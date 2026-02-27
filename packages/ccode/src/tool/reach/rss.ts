import z from "zod"
import { Tool } from "../tool"
import { truncate } from "./utils"
import type { RssFeed, RssItem } from "./types"
import { Log } from "@/util/log"

/**
 * Agent Reach - RSS Tool
 *
 * Read RSS and Atom feeds using built-in parser
 */

const log = Log.create({ service: "reach.rss" })

const DESCRIPTION = `Read RSS and Atom feed content.

Actions:
- read: Fetch and parse an RSS/Atom feed

Examples:
- Read feed: { "url": "https://hnrss.org/frontpage", "limit": 10 }
- Full content: { "url": "https://example.com/feed.xml", "includeContent": true }

This tool uses a built-in XML parser and requires no external dependencies.`

export const RssTool = Tool.define("reach_rss", {
  description: DESCRIPTION,
  parameters: z.object({
    url: z.string().describe("RSS or Atom feed URL"),
    limit: z.number().optional().default(10).describe("Maximum number of items to return"),
    includeContent: z.boolean().optional().default(false).describe("Include full content/description"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "reach_rss",
      patterns: [params.url],
      always: ["*"],
      metadata: {},
    })

    try {
      const response = await fetch(params.url, {
        signal: ctx.abort,
        headers: {
          "User-Agent": "CodeCoder/1.0 (RSS Reader)",
          Accept: "application/rss+xml, application/atom+xml, application/xml, text/xml, */*",
        },
      })

      if (!response.ok) {
        return {
          title: "RSS - Fetch Error",
          metadata: { error: true, httpStatus: response.status } as Record<string, unknown>,
          output: `Failed to fetch feed: HTTP ${response.status} ${response.statusText}`,
        }
      }

      const xml = await response.text()
      const feed = parseRss(xml)

      if (!feed) {
        return {
          title: "RSS - Parse Error",
          metadata: { error: true } as Record<string, unknown>,
          output: "Failed to parse RSS/Atom feed. The content may not be a valid feed.",
        }
      }

      // Apply limit
      const items = feed.items.slice(0, params.limit)
      const output = formatFeed({ ...feed, items }, params.includeContent ?? false)

      return {
        title: `RSS - ${feed.title || "Feed"} (${items.length} items)`,
        metadata: {
          feedTitle: feed.title,
          feedLink: feed.link,
          itemCount: items.length,
          totalItems: feed.items.length,
        } as Record<string, unknown>,
        output,
      }
    } catch (error) {
      log.error("rss fetch failed", { url: params.url, error })
      return {
        title: "RSS - Error",
        metadata: { error: true } as Record<string, unknown>,
        output: `Failed to read feed: ${error instanceof Error ? error.message : "Unknown error"}`,
      }
    }
  },
})

/**
 * Simple RSS/Atom parser using regex
 * Handles both RSS 2.0 and Atom formats
 */
function parseRss(xml: string): RssFeed | null {
  // Detect feed type
  const isAtom = xml.includes("<feed") && xml.includes("xmlns=\"http://www.w3.org/2005/Atom\"")

  if (isAtom) {
    return parseAtom(xml)
  }

  return parseRss2(xml)
}

function parseRss2(xml: string): RssFeed | null {
  try {
    // Extract channel info
    const channelMatch = xml.match(/<channel[^>]*>([\s\S]*?)<\/channel>/)
    if (!channelMatch) return null

    const channel = channelMatch[1]

    const title = extractTag(channel, "title") ?? "Untitled Feed"
    const description = extractTag(channel, "description")
    const link = extractTag(channel, "link")

    // Extract items
    const itemRegex = /<item[^>]*>([\s\S]*?)<\/item>/g
    const items: RssItem[] = []
    let match

    while ((match = itemRegex.exec(channel)) !== null) {
      const itemXml = match[1]
      items.push({
        title: extractTag(itemXml, "title") ?? "Untitled",
        link: extractTag(itemXml, "link"),
        description: extractTag(itemXml, "description"),
        pubDate: extractTag(itemXml, "pubDate"),
        author: extractTag(itemXml, "author") ?? extractTag(itemXml, "dc:creator"),
        guid: extractTag(itemXml, "guid"),
        content: extractTag(itemXml, "content:encoded"),
      })
    }

    return { title, description, link, items }
  } catch (error) {
    log.error("rss2 parse failed", { error })
    return null
  }
}

function parseAtom(xml: string): RssFeed | null {
  try {
    // Extract feed info
    const title = extractTag(xml, "title") ?? "Untitled Feed"
    const subtitle = extractTag(xml, "subtitle")
    const link = extractAtomLink(xml)

    // Extract entries
    const entryRegex = /<entry[^>]*>([\s\S]*?)<\/entry>/g
    const items: RssItem[] = []
    let match

    while ((match = entryRegex.exec(xml)) !== null) {
      const entryXml = match[1]
      items.push({
        title: extractTag(entryXml, "title") ?? "Untitled",
        link: extractAtomLink(entryXml),
        description: extractTag(entryXml, "summary"),
        pubDate: extractTag(entryXml, "published") ?? extractTag(entryXml, "updated"),
        author: extractAtomAuthor(entryXml),
        guid: extractTag(entryXml, "id"),
        content: extractTag(entryXml, "content"),
      })
    }

    return { title, description: subtitle, link, items }
  } catch (error) {
    log.error("atom parse failed", { error })
    return null
  }
}

function extractTag(xml: string, tagName: string): string | undefined {
  // Handle CDATA
  const cdataRegex = new RegExp(`<${tagName}[^>]*><!\\[CDATA\\[([\\s\\S]*?)\\]\\]></${tagName}>`, "i")
  const cdataMatch = xml.match(cdataRegex)
  if (cdataMatch) {
    return decodeHtmlEntities(cdataMatch[1].trim())
  }

  // Handle regular content
  const regex = new RegExp(`<${tagName}[^>]*>([\\s\\S]*?)</${tagName}>`, "i")
  const match = xml.match(regex)
  if (match) {
    return decodeHtmlEntities(match[1].trim())
  }

  return undefined
}

function extractAtomLink(xml: string): string | undefined {
  // Look for link with rel="alternate" or no rel (default is alternate)
  const linkMatch = xml.match(/<link[^>]*href="([^"]*)"[^>]*>/i)
  if (linkMatch) {
    return linkMatch[1]
  }
  return undefined
}

function extractAtomAuthor(xml: string): string | undefined {
  const authorMatch = xml.match(/<author[^>]*>([\s\S]*?)<\/author>/i)
  if (authorMatch) {
    return extractTag(authorMatch[1], "name")
  }
  return undefined
}

function decodeHtmlEntities(text: string): string {
  return text
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#(\d+);/g, (_, code) => String.fromCharCode(Number(code)))
    .replace(/&#x([0-9a-f]+);/gi, (_, code) => String.fromCharCode(parseInt(code, 16)))
}

function formatFeed(feed: RssFeed, includeContent: boolean): string {
  const lines: string[] = [
    `# ${feed.title}`,
  ]

  if (feed.description) {
    lines.push("", truncate(feed.description, 200))
  }

  if (feed.link) {
    lines.push("", `**Link:** ${feed.link}`)
  }

  lines.push("", "---", "")

  for (let i = 0; i < feed.items.length; i++) {
    const item = feed.items[i]
    lines.push(`## ${i + 1}. ${item.title}`)

    if (item.link) {
      lines.push(`- **Link:** ${item.link}`)
    }

    if (item.pubDate) {
      lines.push(`- **Date:** ${item.pubDate}`)
    }

    if (item.author) {
      lines.push(`- **Author:** ${item.author}`)
    }

    if (includeContent) {
      const content = item.content ?? item.description
      if (content) {
        // Strip HTML tags for readability
        const text = content.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim()
        lines.push("", text)
      }
    } else if (item.description) {
      // Show truncated description
      const text = item.description.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim()
      lines.push("", truncate(text, 200))
    }

    lines.push("")
  }

  return lines.join("\n")
}
