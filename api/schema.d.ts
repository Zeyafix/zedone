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
    "/v1/commands/card.create": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.edit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_edit"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_history"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_list"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.read": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_read"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_restore"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.trash": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_trash"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.create": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_list"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.read": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_read"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.rename": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_rename"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_restore"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.trash": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_trash"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.create": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["list"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.read": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["read"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.rename": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["rename"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["restore"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.trash": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["trash"];
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
        AccessScope: components["schemas"]["AdministrationScope"] | components["schemas"]["AdministrationAndSpacesScope"] | components["schemas"]["SpacesScope"];
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
        AdministrationAndSpacesKind: "administration_and_spaces";
        AdministrationAndSpacesScope: {
            kind: components["schemas"]["AdministrationAndSpacesKind"];
            spaces: {
                [key: string]: components["schemas"]["AccessMode"];
            };
        };
        /** @enum {string} */
        AdministrationKind: "administration";
        AdministrationScope: {
            kind: components["schemas"]["AdministrationKind"];
        };
        CardHistory: {
            card: string;
            next?: string | null;
            space: string;
            version: components["schemas"]["StateVersion"];
            versions: components["schemas"]["CardTextVersion"][];
        };
        CardList: {
            cards: components["schemas"]["CardSnapshot"][];
        };
        CardSnapshot: {
            card: string;
            columns: string[];
            completed: boolean[];
            deletion_groups: string[];
            descriptions: string[];
            positions: number[];
            space: string;
            state: components["schemas"]["SpaceState"];
            tags: string[];
            titles: string[];
            version: components["schemas"]["StateVersion"];
        };
        CardTextVersion: {
            description: string;
            id: string;
            /**
             * Format: int64
             * @description Local causal sequence; independent branches may share a sequence.
             */
            sequence: number;
            subject: string;
            title: string;
        };
        ColumnList: {
            columns: components["schemas"]["ColumnSnapshot"][];
        };
        ColumnSnapshot: {
            column: string;
            deletion_groups: string[];
            /** @description Unknown/future contents block trash and restore until supported. */
            empty: boolean;
            names: string[];
            /** @description All distinct position registers; conflicting values are never overwritten. */
            positions: number[];
            space: string;
            state: components["schemas"]["SpaceState"];
            version: components["schemas"]["StateVersion"];
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
        CommandRequest_CreateCard: {
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
                card: string;
                column: string;
                description: string;
                space: string;
                title: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_CreateColumn: {
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
                column: string;
                name: string;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_CreateSpace: {
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
                name: string;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_EditCard: {
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
                card: string;
                description: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
                title: string;
            };
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
        CommandRequest_ListCards: {
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
                include_trashed: boolean;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_ListColumns: {
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
                include_trashed: boolean;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_ListSpaces: {
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
                include_trashed: boolean;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_ReadCard: {
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
                card: string;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_ReadCardHistory: {
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
                after?: string | null;
                card: string;
                expected?: components["schemas"]["StateVersion"] | null;
                /** Format: int32 */
                limit: number;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_ReadColumn: {
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
                column: string;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_ReadSpace: {
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
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_RenameColumn: {
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
                column: string;
                expected: components["schemas"]["StateVersion"];
                name: string;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_RenameSpace: {
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
                name: string;
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_RestoreCard: {
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
                card: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_RestoreColumn: {
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
                column: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_RestoreSpace: {
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
                space: string;
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
        CommandRequest_TrashCard: {
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
                card: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_TrashColumn: {
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
                column: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        /**
         * @description Preserve this entire request, including the ID, for every delivery of an
         *     intention. New intentions require new client-generated cryptographic IDs.
         *     Concrete command types define all targets, parameters and original versions.
         *     The Value specialization in OpenAPI describes only this envelope, not an
         *     implemented operation or a substitute for concrete command validation.
         */
        CommandRequest_TrashSpace: {
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
                space: string;
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
        CommandResponse_CardHistory: components["schemas"]["CommandSuccess_CardHistory"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_CardList: components["schemas"]["CommandSuccess_CardList"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_CardSnapshot: components["schemas"]["CommandSuccess_CardSnapshot"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_ColumnList: components["schemas"]["CommandSuccess_ColumnList"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_ColumnSnapshot: components["schemas"]["CommandSuccess_ColumnSnapshot"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_SpaceList: components["schemas"]["CommandSuccess_SpaceList"] | components["schemas"]["CommandFailure"];
        /**
         * @description Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
         *     A success confirms the original completion, not the current object state.
         *     Construct success only AFTER the storage transaction commits. Cancellation,
         *     a lost response or uncertain commit must never be converted to success.
         */
        CommandResponse_SpaceSnapshot: components["schemas"]["CommandSuccess_SpaceSnapshot"] | components["schemas"]["CommandFailure"];
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
        CommandSuccess_CardHistory: {
            replayed: boolean;
            result: {
                card: string;
                next?: string | null;
                space: string;
                version: components["schemas"]["StateVersion"];
                versions: components["schemas"]["CardTextVersion"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_CardList: {
            replayed: boolean;
            result: {
                cards: components["schemas"]["CardSnapshot"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_CardSnapshot: {
            replayed: boolean;
            result: {
                card: string;
                columns: string[];
                completed: boolean[];
                deletion_groups: string[];
                descriptions: string[];
                positions: number[];
                space: string;
                state: components["schemas"]["SpaceState"];
                tags: string[];
                titles: string[];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_ColumnList: {
            replayed: boolean;
            result: {
                columns: components["schemas"]["ColumnSnapshot"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_ColumnSnapshot: {
            replayed: boolean;
            result: {
                column: string;
                deletion_groups: string[];
                /** @description Unknown/future contents block trash and restore until supported. */
                empty: boolean;
                names: string[];
                /** @description All distinct position registers; conflicting values are never overwritten. */
                positions: number[];
                space: string;
                state: components["schemas"]["SpaceState"];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SpaceList: {
            replayed: boolean;
            result: {
                spaces: components["schemas"]["SpaceSnapshot"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SpaceSnapshot: {
            replayed: boolean;
            result: {
                deletion_groups: string[];
                /** @description This slice only trashes/restores spaces without domain content. */
                empty: boolean;
                /** @description All distinct current names; multiple names require conflict resolution. */
                names: string[];
                space: string;
                state: components["schemas"]["SpaceState"];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_Value: {
            replayed: boolean;
            result: unknown;
            status: components["schemas"]["SuccessStatus"];
        };
        CreateCard: {
            card: string;
            column: string;
            description: string;
            space: string;
            title: string;
        };
        CreateColumn: {
            column: string;
            name: string;
            space: string;
        };
        CreateSpace: {
            name: string;
            space: string;
        };
        EditCard: {
            card: string;
            description: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
            title: string;
        };
        /**
         * @description Shared machine-readable errors, without credentials, SQL or platform details.
         *
         *     Codes describe semantics, not HTTP statuses or retry policy. A stale request
         *     requires a fresh read and a new explicit action. An unknown outcome requires
         *     checking/replaying the original command, never inventing a new command ID.
         * @enum {string}
         */
        ErrorCode: "invalid_request" | "access_denied" | "not_found" | "stale_state" | "conflict" | "command_mismatch" | "unavailable" | "outcome_unknown";
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
        ListCards: {
            include_trashed: boolean;
            space: string;
        };
        ListColumns: {
            include_trashed: boolean;
            space: string;
        };
        ListSpaces: {
            include_trashed: boolean;
        };
        ReadCard: {
            card: string;
            space: string;
        };
        ReadCardHistory: {
            after?: string | null;
            card: string;
            expected?: components["schemas"]["StateVersion"] | null;
            /** Format: int32 */
            limit: number;
            space: string;
        };
        ReadColumn: {
            column: string;
            space: string;
        };
        ReadSpace: {
            space: string;
        };
        RenameColumn: {
            column: string;
            expected: components["schemas"]["StateVersion"];
            name: string;
            space: string;
        };
        RenameSpace: {
            expected: components["schemas"]["StateVersion"];
            name: string;
            space: string;
        };
        RestoreCard: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        RestoreColumn: {
            column: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        RestoreSpace: {
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        RevokeAccess: {
            expected: components["schemas"]["StateVersion"];
            subject: string;
        };
        SpaceList: {
            spaces: components["schemas"]["SpaceSnapshot"][];
        };
        SpaceSnapshot: {
            deletion_groups: string[];
            /** @description This slice only trashes/restores spaces without domain content. */
            empty: boolean;
            /** @description All distinct current names; multiple names require conflict resolution. */
            names: string[];
            space: string;
            state: components["schemas"]["SpaceState"];
            version: components["schemas"]["StateVersion"];
        };
        /** @enum {string} */
        SpaceState: "active" | "trashed" | "conflicted";
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
        TrashCard: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        TrashColumn: {
            column: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        TrashSpace: {
            expected: components["schemas"]["StateVersion"];
            space: string;
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
    card_create: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CreateCard"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardSnapshot"];
                };
            };
        };
    };
    card_edit: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_EditCard"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardSnapshot"];
                };
            };
        };
    };
    card_history: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ReadCardHistory"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardHistory"];
                };
            };
        };
    };
    card_list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListCards"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardList"];
                };
            };
        };
    };
    card_read: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ReadCard"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardSnapshot"];
                };
            };
        };
    };
    card_restore: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RestoreCard"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardSnapshot"];
                };
            };
        };
    };
    card_trash: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_TrashCard"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardSnapshot"];
                };
            };
        };
    };
    column_create: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CreateColumn"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnSnapshot"];
                };
            };
        };
    };
    column_list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListColumns"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnList"];
                };
            };
        };
    };
    column_read: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ReadColumn"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnSnapshot"];
                };
            };
        };
    };
    column_rename: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RenameColumn"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnSnapshot"];
                };
            };
        };
    };
    column_restore: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RestoreColumn"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnSnapshot"];
                };
            };
        };
    };
    column_trash: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_TrashColumn"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnSnapshot"];
                };
            };
        };
    };
    create: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CreateSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceSnapshot"];
                };
            };
        };
    };
    list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListSpaces"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceList"];
                };
            };
        };
    };
    read: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ReadSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceSnapshot"];
                };
            };
        };
    };
    rename: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RenameSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceSnapshot"];
                };
            };
        };
    };
    restore: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RestoreSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceSnapshot"];
                };
            };
        };
    };
    trash: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_TrashSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceSnapshot"];
                };
            };
        };
    };
}
