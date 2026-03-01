import { createProviderToolFactoryWithOutputSchema } from "@ai-sdk/provider-utils"
import { z } from "zod/v4"

export const webSearchOutputSchema = z.object({
  results: z
    .array(
      z.object({
        title: z.string(),
        url: z.string(),
        snippet: z.string().optional(),
        content: z.string().optional(),
      }),
    )
    .nullable(),
})

export const webSearchArgsSchema = z.object({
  filters: z
    .object({
      allowedDomains: z.array(z.string()).optional(),
    })
    .optional(),

  searchContextSize: z.enum(["low", "medium", "high"]).optional(),

  userLocation: z
    .object({
      type: z.literal("approximate"),
      country: z.string().optional(),
      city: z.string().optional(),
      region: z.string().optional(),
      timezone: z.string().optional(),
    })
    .optional(),
})

export const webSearchToolFactory = createProviderToolFactoryWithOutputSchema<
  {
    // Web search doesn't take input parameters - it's controlled by the prompt
  },
  {
    /**
     * The results of the web search.
     */
    results:
      | null
      | {
          /**
           * The title of the search result.
           */
          title: string

          /**
           * The URL of the search result.
           */
          url: string

          /**
           * A snippet from the search result.
           */
          snippet?: string

          /**
           * The full content of the page (if retrieved).
           */
          content?: string
        }[]
  },
  {
    /**
     * Filters for the search.
     */
    filters?: {
      /**
       * Allowed domains for the search.
       * If not provided, all domains are allowed.
       * Subdomains of the provided domains are allowed as well.
       */
      allowedDomains?: string[]
    }

    /**
     * Search context size to use for the web search.
     * - high: Most comprehensive context, highest cost, slower response
     * - medium: Balanced context, cost, and latency (default)
     * - low: Least context, lowest cost, fastest response
     */
    searchContextSize?: "low" | "medium" | "high"

    /**
     * User location information to provide geographically relevant search results.
     */
    userLocation?: {
      /**
       * Type of location (always 'approximate')
       */
      type: "approximate"
      /**
       * Two-letter ISO country code (e.g., 'US', 'GB')
       */
      country?: string
      /**
       * City name (free text, e.g., 'Minneapolis')
       */
      city?: string
      /**
       * Region name (free text, e.g., 'Minnesota')
       */
      region?: string
      /**
       * IANA timezone (e.g., 'America/Chicago')
       */
      timezone?: string
    }
  }
>({
  id: "openai.web_search",
  name: "web_search",
  inputSchema: z.object({
    action: z
      .discriminatedUnion("type", [
        z.object({
          type: z.literal("search"),
          query: z.string().nullish(),
        }),
        z.object({
          type: z.literal("open_page"),
          url: z.string(),
        }),
        z.object({
          type: z.literal("find"),
          url: z.string(),
          pattern: z.string(),
        }),
      ])
      .nullish(),
  }),
  outputSchema: webSearchOutputSchema,
})

export const webSearch = (
  args: Parameters<typeof webSearchToolFactory>[0] = {}, // default
) => {
  return webSearchToolFactory(args)
}
