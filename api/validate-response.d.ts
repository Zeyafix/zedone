import type { components } from "./schema";
export type Operation = "access.inspect" | "access.revoke" | "space.create" | "space.list" | "space.read" | "space.rename" | "space.restore" | "space.trash";
export type Parameters = {
  "access.inspect": components["schemas"]["CommandRequest_InspectAccess"]["parameters"];
  "access.revoke": components["schemas"]["CommandRequest_RevokeAccess"]["parameters"];
  "space.create": components["schemas"]["CommandRequest_CreateSpace"]["parameters"];
  "space.list": components["schemas"]["CommandRequest_ListSpaces"]["parameters"];
  "space.read": components["schemas"]["CommandRequest_ReadSpace"]["parameters"];
  "space.rename": components["schemas"]["CommandRequest_RenameSpace"]["parameters"];
  "space.restore": components["schemas"]["CommandRequest_RestoreSpace"]["parameters"];
  "space.trash": components["schemas"]["CommandRequest_TrashSpace"]["parameters"];
};
export type Response = components["schemas"]["CommandResponse_AccessSnapshot"] | components["schemas"]["CommandResponse_SpaceSnapshot"] | components["schemas"]["CommandResponse_SpaceList"];
export const operations: readonly Operation[];
export default function validate(value: unknown, operation?: Operation): value is Response;
