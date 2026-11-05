export interface paths {
    "/v1/commands/access.inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["inspect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/access.revoke": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["revoke"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @enum {string} */
        AccessMode: "read" | "read_write";
        AccessScope: components["schemas"]["AdministrationScope"] | components["schemas"]["SpacesScope"];
        /**
         * @description No keys or verifiers leave storage. Version includes retired/current key
         *     identities internally so a regrant cannot revive a stale revocation request.
         */
        AccessSnapshot: {
            /** Format: int64 */
            expires_at_unix_ms?: number | null;
            revoked: boolean;
            scope: components["schemas"]["AccessScope"];
            subject: string;
            version: components["schemas"]["StateVersion"];
        };
        /** @enum {string} */
        AdministrationKind: "administration";
        AdministrationScope: {
            kind: components["schemas"]["AdministrationKind"];
        };
        CommandError: {
            code: components["schemas"]["ErrorCode"];
        };
        CommandFailure: {
            error: components["schemas"]["CommandError"];
            status: components["schemas"]["ErrorStatus"];
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_InspectAccess: {
            id: [
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number
            ];
            operation: string;
            parameters: {
                subject: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_RevokeAccess: {
            id: [
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number
            ];
            operation: string;
            parameters: {
                expected: components["schemas"]["StateVersion"];
                subject: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_Value: {
            id: [
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number,
                number
            ];
            operation: string;
            parameters: unknown;
        };
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_AccessSnapshot: components["schemas"]["CommandSuccess_AccessSnapshot"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_Value: components["schemas"]["CommandSuccess_Value"] | components["schemas"]["CommandFailure"];
        CommandSuccess_AccessSnapshot: {
            replayed: boolean;
            /**
             * @description No keys or verifiers leave storage. Version includes retired/current key
             *     identities internally so a regrant cannot revive a stale revocation request.
             */
            result: {
                /** Format: int64 */
                expires_at_unix_ms?: number | null;
                revoked: boolean;
                scope: components["schemas"]["AccessScope"];
                subject: string;
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_Value: {
            replayed: boolean;
            result: unknown;
            status: components["schemas"]["SuccessStatus"];
        };
        /**
         * @description Shared machine-readable errors, without credentials, SQL or platform details.
         *
         *     Codes describe semantics, not HTTP statuses or retry policy. A stale request
         *     requires a fresh read and a new explicit action. An unknown outcome requires
         *     checking/replaying the original command, never inventing a new command ID.
         * @enum {string}
         */
        ErrorCode: "invalid_request" | "access_denied" | "not_found" | "stale_state" | "command_mismatch" | "unavailable" | "outcome_unknown";
        /** @enum {string} */
        ErrorStatus: "error";
        InspectAccess: {
            subject: string;
        };
        /**
         * @description Tauri framing preserves the original JSON string, including duplicate
         *     fields for the concrete decoder to reject. Credentials are a separate IPC
         *     argument, never part of the command or a saved client intention.
         */
        IpcInvocation: {
            operation: string;
            request: string;
        };
        RevokeAccess: {
            expected: components["schemas"]["StateVersion"];
            subject: string;
        };
        /** @enum {string} */
        SpacesKind: "spaces";
        SpacesScope: {
            kind: components["schemas"]["SpacesKind"];
            spaces: {
                [key: string]: components["schemas"]["AccessMode"];
            };
        };
        StateVersion: number[];
        /** @enum {string} */
        SuccessStatus: "ok";
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    inspect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectAccess"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_AccessSnapshot"];
                };
            };
        };
    };
    revoke: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RevokeAccess"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_AccessSnapshot"];
                };
            };
        };
    };
}
