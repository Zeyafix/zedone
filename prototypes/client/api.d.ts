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
            column: number;
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
        HttpSessionRequest: {
            access?: components["schemas"]["TestAccess"];
            action: "start";
            address: string;
            port: number;
        } | {
            action: "status";
        } | {
            action: "stop";
        };
        HttpSessionStatus: {
            access?: components["schemas"]["TestAccess"] | null;
            endpoint?: string | null;
            recovering: boolean;
            token?: string | null;
        };
        Invitation: {
            address: string;
            code: string;
        };
        InvitationError: "invalid_data";
        InvitationRequest: {
            source: "qr";
            text: string;
        } | {
            address: string;
            code: string;
            source: "manual";
        };
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
        Space: "A" | "B";
        StorageError: {
            message: string;
        };
        StorageRequest: {
            action: "seed";
            space: components["schemas"]["Space"];
        } | {
            action: "read";
            query: string;
            space: components["schemas"]["Space"];
        } | {
            action: "snapshot";
            space: components["schemas"]["Space"];
        } | {
            action: "restore";
            snapshot: components["schemas"]["Snapshot"];
        } | {
            action: "delete";
            space: components["schemas"]["Space"];
        } | {
            action: "move_card";
            card: number;
            column: number;
            expected: components["schemas"]["Snapshot"];
            position: number;
        } | {
            action: "move_column";
            column: number;
            expected: components["schemas"]["Snapshot"];
            position: number;
        };
        StorageResponse: {
            snapshot?: components["schemas"]["Snapshot"] | null;
        };
        TestAccess: "read" | "write";
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
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Invitation"];
                };
            };
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
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["QrResponse"];
                };
            };
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
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StorageResponse"];
                };
            };
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
