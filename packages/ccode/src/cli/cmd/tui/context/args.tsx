import { createSimpleContext } from "./helper"

export interface Args {
  model?: string
  agent?: string
  mode?: string
  prompt?: string
  continue?: boolean
  sessionID?: string
}

export const { use: useArgs, provider: ArgsProvider } = createSimpleContext({
  name: "Args",
  init: (props: Args) => props,
})
