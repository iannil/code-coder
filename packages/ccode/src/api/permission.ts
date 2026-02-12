import { Permission } from "@/permission"
import { PermissionNext } from "@/permission/next"
import { fn } from "@/util/fn"
import z from "zod"

export namespace LocalPermission {
  export const list = () => Permission.list()

  export const respond = fn(
    z.object({
      sessionID: z.string(),
      permissionID: z.string(),
      response: Permission.Response,
      message: z.string().optional(),
    }),
    Permission.respond,
  )

  export const reply = fn(
    z.object({
      requestID: z.string(),
      reply: PermissionNext.Reply,
      message: z.string().optional(),
    }),
    PermissionNext.reply,
  )
}
