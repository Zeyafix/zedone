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
    "/prototype/invitation": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * The same inspection for camera content, manual input and direct agent calls.
         *     These fields deliberately do not specify the future pairing protocol.
         */
        post: operations["invitation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/prototype/qr": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Encode every UTF-8 byte, without normalization, truncation or network access. */
        post: operations["qr"];
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
        Invitation: {
            address: string;
            code: string;
        };
        /** @enum {string} */
        InvitationError: "invalid_data";
        InvitationRequest: {
            /** @enum {string} */
            source: "qr";
            text: string;
        } | {
            address: string;
            code: string;
            /** @enum {string} */
            source: "manual";
        };
        /** @enum {string} */
        QrError: "too_long";
        QrRequest: {
            text: string;
        };
        QrResponse: {
            modules: boolean[][];
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
        } | {
            /** @enum {string} */
            action: "move_card";
            /** Format: int32 */
            card: number;
            /** Format: int32 */
            column: number;
            expected: components["schemas"]["Snapshot"];
            /** Format: int32 */
            position: number;
        } | {
            /** @enum {string} */
            action: "move_column";
            /** Format: int32 */
            column: number;
            expected: components["schemas"]["Snapshot"];
            /** Format: int32 */
            position: number;
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
    invitation: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InvitationRequest"];
            };
        };
        responses: {
            /** @description Synthetic fields, unchanged; no connection */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Invitation"];
                };
            };
            /** @description Expected nonempty synthetic address/code */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvitationError"];
                };
            };
        };
    };
    qr: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["QrRequest"];
            };
        };
        responses: {
            /** @description QR modules without quiet zone */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["QrResponse"];
                };
            };
            /** @description Entire text cannot fit in a QR */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["QrError"];
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
