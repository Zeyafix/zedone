export interface paths {
    "/prototype/echo": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Return synthetic text unchanged, without storing it. */
        post: operations["echo"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/prototype/storage": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Contract marker: HTTP and Android IPC call Storage::execute asynchronously. */
        post: operations["storage"];
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
        Card: {
            /** Format: int32 */
            column: number;
            /** Format: int32 */
            id: number;
            text: string;
            title: string;
        };
        EchoRequest: {
            text: string;
        };
        EchoResponse: {
            text: string;
        };
        Snapshot: {
            cards: components["schemas"]["Card"][];
            columns: string[];
            space: components["schemas"]["Space"];
        };
        /** @enum {string} */
        Space: "A" | "B";
        StorageError: {
            message: string;
        };
        StorageRequest: {
            /** @enum {string} */
            action: "seed";
            space: components["schemas"]["Space"];
        } | {
            /** @enum {string} */
            action: "read";
            query: string;
            space: components["schemas"]["Space"];
        } | {
            /** @enum {string} */
            action: "snapshot";
            space: components["schemas"]["Space"];
        } | {
            /** @enum {string} */
            action: "restore";
            snapshot: components["schemas"]["Snapshot"];
        } | {
            /** @enum {string} */
            action: "delete";
            space: components["schemas"]["Space"];
        };
        StorageResponse: {
            snapshot?: components["schemas"]["Snapshot"] | null;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    echo: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EchoRequest"];
            };
        };
        responses: {
            /** @description Text returned unchanged */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EchoResponse"];
                };
            };
        };
    };
    storage: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StorageRequest"];
            };
        };
        responses: {
            /** @description Committed synthetic result or consistent read */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StorageResponse"];
                };
            };
            /** @description Storage failed; success not confirmed */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StorageError"];
                };
            };
        };
    };
}
