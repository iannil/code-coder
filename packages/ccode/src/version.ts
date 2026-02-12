import { Flag } from "@/flag/flag"

declare global {
  const CCODE_VERSION: string
  const CCODE_CHANNEL: string
}

export const VERSION = typeof CCODE_VERSION === "string" ? CCODE_VERSION : "local"
export const CHANNEL = typeof CCODE_CHANNEL === "string" ? CCODE_CHANNEL : "local"
export const USER_AGENT = `ccode/${CHANNEL}/${VERSION}/${Flag.CCODE_CLIENT}`

export function isLocal() {
  return CHANNEL === "local"
}

export function isPreview() {
  return CHANNEL !== "latest"
}
