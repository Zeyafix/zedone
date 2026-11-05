export type paths = Record<string, never>;
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
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
        CommandResponse_Value: components["schemas"]["CommandSuccess_Value"] | components["schemas"]["CommandFailure"];
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
export type operations = Record<string, never>;
