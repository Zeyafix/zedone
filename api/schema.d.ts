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
    "/v1/commands/access.issue": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["issue_access"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/access.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["list_access"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/access.replace": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["replace_access"];
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
    "/v1/commands/card.autosave": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_autosave"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.begin_editing": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_begin_editing"];
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
    "/v1/commands/card.finish_editing": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_finish_editing"];
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
    "/v1/commands/card.inspect_placements": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_inspect_placements"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.inspect_tags": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_inspect_tags"];
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
    "/v1/commands/card.move": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_move"];
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
    "/v1/commands/card.resolve_deletion": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_resolve_deletion"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.resolve_placement": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_resolve_placement"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.resolve_tag": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_resolve_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.resolve_text": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_resolve_text"];
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
    "/v1/commands/card.restore_text": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_restore_text"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_search"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.set_completed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_set_completed"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/card.set_tag": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["card_set_tag"];
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
    "/v1/commands/column.inspect_positions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_inspect_positions"];
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
    "/v1/commands/column.move": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_move"];
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
    "/v1/commands/column.resolve_deletion": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_resolve_deletion"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.resolve_name": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_resolve_name"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/column.resolve_position": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["column_resolve_position"];
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
    "/v1/commands/history.clear": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["history_clear"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/history.inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["history_inspect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/invitation.cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["invitation_cancel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/invitation.create": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["invitation_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/invitation.inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["invitation_inspect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/invitation.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["invitation_list"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/pairing.cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pairing_cancel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/pairing.confirm": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pairing_confirm"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/pairing.inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pairing_inspect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/pairing.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pairing_list"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/pairing.reject": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pairing_reject"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/pairing.unlink": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pairing_unlink"];
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
    "/v1/commands/space.export": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["space_export"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["space_import"];
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
    "/v1/commands/space.resolve_deletion": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["space_resolve_deletion"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/space.resolve_name": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["space_resolve_name"];
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
    "/v1/commands/sync.catalog": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sync_catalog"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/sync.inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sync_inspect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/sync.pause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sync_pause"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/sync.pull": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sync_pull"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/sync.push": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sync_push"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/sync.select": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sync_select"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.create": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_delete"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_list"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.read": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_read"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.rename": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_rename"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.resolve_deletion": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_resolve_deletion"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/tag.resolve_names": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["tag_resolve_names"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/trash.inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["trash_inspect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/trash.purge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["trash_purge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/commands/trash.resolve_purge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["trash_resolve_purge"];
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
        AccessList: {
            entries: components["schemas"]["AccessSnapshot"][];
            next?: string | null;
        };
        AccessMode: "read" | "read_write";
        AccessScope: components["schemas"]["AdministrationScope"] | components["schemas"]["AdministrationAndSpacesScope"] | components["schemas"]["SpacesScope"];
        AccessSnapshot: {
            expires_at_unix_ms?: number | null;
            revoked: boolean;
            scope: components["schemas"]["AccessScope"];
            subject: string;
            version: components["schemas"]["StateVersion"];
        };
        AdministrationAndSpacesKind: "administration_and_spaces";
        AdministrationAndSpacesScope: {
            kind: components["schemas"]["AdministrationAndSpacesKind"];
            spaces: {
                [key: string]: components["schemas"]["AccessMode"];
            };
        };
        AdministrationKind: "administration";
        AdministrationScope: {
            kind: components["schemas"]["AdministrationKind"];
        };
        AutosaveCard: {
            card: string;
            description: string;
            expected: components["schemas"]["StateVersion"];
            session: string;
            space: string;
            title: string;
        };
        BeginCardEditing: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        CancelInvitation: {
            expected: components["schemas"]["StateVersion"];
            invitation: string;
        };
        CancelPairing: {
            expected: components["schemas"]["StateVersion"];
            request: string;
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
        CardPlacement: {
            column: string;
            rank: components["schemas"]["Rank"];
        };
        CardPlacements: {
            card: components["schemas"]["CardSnapshot"];
            choices: components["schemas"]["PlacementChoice"][];
        };
        CardSnapshot: {
            card: string;
            columns: string[];
            completed: boolean[];
            deletion_conflict?: boolean;
            deletion_groups: string[];
            descriptions: string[];
            placements?: components["schemas"]["CardPlacement"][];
            positions: components["schemas"]["Rank"][];
            space: string;
            state: components["schemas"]["SpaceState"];
            tags: string[];
            titles: string[];
            version: components["schemas"]["StateVersion"];
        };
        CardTags: {
            assignments: components["schemas"]["TagAssignment"][];
            card: components["schemas"]["CardSnapshot"];
        };
        CardTextVersion: {
            description: string;
            id: string;
            sequence: number;
            subject: string;
            title: string;
        };
        CatalogSync: Record<string, never>;
        ClearHistory: {
            card?: string | null;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        ColumnList: {
            columns: components["schemas"]["ColumnSnapshot"][];
        };
        ColumnPositionChoice: {
            before?: string | null;
            position: components["schemas"]["Rank"];
        };
        ColumnPositions: {
            choices: components["schemas"]["ColumnPositionChoice"][];
            column: components["schemas"]["ColumnSnapshot"];
        };
        ColumnSnapshot: {
            column: string;
            contents?: components["schemas"]["CardSnapshot"][] | null;
            deletion_groups: string[];
            empty: boolean;
            names: string[];
            positions: components["schemas"]["Rank"][];
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
        CommandRequest_AutosaveCard: {
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
                session: string;
                space: string;
                title: string;
            };
        };
        CommandRequest_BeginCardEditing: {
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
        CommandRequest_CancelInvitation: {
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
                invitation: string;
            };
        };
        CommandRequest_CancelPairing: {
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
                request: string;
            };
        };
        CommandRequest_CatalogSync: {
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
            parameters: Record<string, never>;
        };
        CommandRequest_ClearHistory: {
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
                card?: string | null;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        CommandRequest_ConfirmPairing: {
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
                expires_at_unix_ms?: number | null;
                request: string;
                scope: components["schemas"]["AccessScope"];
            };
        };
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
        CommandRequest_CreateInvitation: {
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
                invitation: string;
                verifier: components["schemas"]["InvitationVerifier"];
            };
        };
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
        CommandRequest_CreateTag: {
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
                tag: string;
            };
        };
        CommandRequest_DeleteTag: {
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
                tag: string;
            };
        };
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
        CommandRequest_ExportSpace: {
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
        CommandRequest_FinishCardEditing: {
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
                session: string;
                space: string;
            };
        };
        CommandRequest_ImportSpace: {
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
                archive: string;
            };
        };
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
        CommandRequest_InspectCardPlacements: {
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
        CommandRequest_InspectCardTags: {
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
        CommandRequest_InspectColumnPositions: {
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
        CommandRequest_InspectHistory: {
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
                card?: string | null;
                space: string;
            };
        };
        CommandRequest_InspectInvitation: {
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
                invitation: string;
            };
        };
        CommandRequest_InspectPairing: {
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
                request: string;
            };
        };
        CommandRequest_InspectSync: {
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
                pairing: string;
                space: string;
            };
        };
        CommandRequest_InspectTrash: {
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
                selection: components["schemas"]["TrashSelection"];
                space: string;
            };
        };
        CommandRequest_IssueAccess: {
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
                expires_at_unix_ms?: number | null;
                scope: components["schemas"]["AccessScope"];
                subject: string;
                verifier: components["schemas"]["KeyVerifier"];
            };
        };
        CommandRequest_ListAccess: {
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
            };
        };
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
        CommandRequest_ListInvitations: {
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
            };
        };
        CommandRequest_ListPairings: {
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
            };
        };
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
        CommandRequest_ListTags: {
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
                include_deleted: boolean;
                space: string;
            };
        };
        CommandRequest_MoveCard: {
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
                before?: string | null;
                card: string;
                column: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        CommandRequest_MoveColumn: {
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
                before?: string | null;
                column: string;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        CommandRequest_PauseSync: {
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
                pairing: string;
                paused: boolean;
                space: string;
            };
        };
        CommandRequest_PullSync: {
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
                known: number[][];
                protocol: number;
                space: string;
            };
        };
        CommandRequest_PurgeTrash: {
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
                selection: components["schemas"]["TrashSelection"];
                space: string;
            };
        };
        CommandRequest_PushSync: {
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
                base: number[][];
                changes: string[];
                protocol: number;
                space: string;
            };
        };
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
                limit: number;
                space: string;
            };
        };
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
        CommandRequest_ReadTag: {
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
                tag: string;
            };
        };
        CommandRequest_RejectPairing: {
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
                request: string;
            };
        };
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
        CommandRequest_RenameTag: {
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
                tag: string;
            };
        };
        CommandRequest_ReplaceAccess: {
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
                expires_at_unix_ms?: number | null;
                scope: components["schemas"]["AccessScope"];
                subject: string;
                verifier: components["schemas"]["KeyVerifier"];
            };
        };
        CommandRequest_RequestPairing: {
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
                access_verifier: components["schemas"]["KeyVerifier"];
                invitation: string;
                label: string;
                request: string;
            };
        };
        CommandRequest_ResolveCardDeletion: {
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
                keep: boolean;
                space: string;
            };
        };
        CommandRequest_ResolveCardPlacement: {
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
                placement: components["schemas"]["CardPlacement"];
                space: string;
            };
        };
        CommandRequest_ResolveCardTag: {
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
                expected_tag: components["schemas"]["StateVersion"];
                present: boolean;
                space: string;
                tag: string;
            };
        };
        CommandRequest_ResolveCardText: {
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
        CommandRequest_ResolveColumnDeletion: {
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
                keep: boolean;
                space: string;
            };
        };
        CommandRequest_ResolveColumnName: {
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
        CommandRequest_ResolveColumnPosition: {
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
                position: components["schemas"]["Rank"];
                space: string;
            };
        };
        CommandRequest_ResolvePurge: {
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
                keep: boolean;
                selection: components["schemas"]["TrashSelection"];
                space: string;
            };
        };
        CommandRequest_ResolveSpaceDeletion: {
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
                keep: boolean;
                space: string;
            };
        };
        CommandRequest_ResolveSpaceName: {
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
        CommandRequest_ResolveTagDeletion: {
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
                keep: boolean;
                space: string;
                tag: string;
            };
        };
        CommandRequest_ResolveTagNames: {
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
                names: components["schemas"]["TagNameChoice"][];
                space: string;
                tag: string;
            };
        };
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
                column?: string | null;
                expected: components["schemas"]["StateVersion"];
                expected_column?: components["schemas"]["StateVersion"] | null;
                space: string;
            };
        };
        CommandRequest_RestoreCardText: {
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
                text_version: string;
            };
        };
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
        CommandRequest_SearchCards: {
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
                query: string;
                scope: components["schemas"]["SearchScope"];
            };
        };
        CommandRequest_SelectSync: {
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
                pairing: string;
                selected: boolean;
                space: string;
            };
        };
        CommandRequest_SetCardCompleted: {
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
                completed: boolean;
                expected: components["schemas"]["StateVersion"];
                space: string;
            };
        };
        CommandRequest_SetCardTag: {
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
                expected_tag: components["schemas"]["StateVersion"];
                present: boolean;
                space: string;
                tag: string;
            };
        };
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
        CommandRequest_UnlinkPairing: {
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
                request: string;
            };
        };
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
        CommandResponse_AccessList: components["schemas"]["CommandSuccess_AccessList"] | components["schemas"]["CommandFailure"];
        CommandResponse_AccessSnapshot: components["schemas"]["CommandSuccess_AccessSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_CardHistory: components["schemas"]["CommandSuccess_CardHistory"] | components["schemas"]["CommandFailure"];
        CommandResponse_CardList: components["schemas"]["CommandSuccess_CardList"] | components["schemas"]["CommandFailure"];
        CommandResponse_CardPlacements: components["schemas"]["CommandSuccess_CardPlacements"] | components["schemas"]["CommandFailure"];
        CommandResponse_CardSnapshot: components["schemas"]["CommandSuccess_CardSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_CardTags: components["schemas"]["CommandSuccess_CardTags"] | components["schemas"]["CommandFailure"];
        CommandResponse_ColumnList: components["schemas"]["CommandSuccess_ColumnList"] | components["schemas"]["CommandFailure"];
        CommandResponse_ColumnPositions: components["schemas"]["CommandSuccess_ColumnPositions"] | components["schemas"]["CommandFailure"];
        CommandResponse_ColumnSnapshot: components["schemas"]["CommandSuccess_ColumnSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_HistoryPreview: components["schemas"]["CommandSuccess_HistoryPreview"] | components["schemas"]["CommandFailure"];
        CommandResponse_ImportedSpace: components["schemas"]["CommandSuccess_ImportedSpace"] | components["schemas"]["CommandFailure"];
        CommandResponse_InvitationList: components["schemas"]["CommandSuccess_InvitationList"] | components["schemas"]["CommandFailure"];
        CommandResponse_InvitationSnapshot: components["schemas"]["CommandSuccess_InvitationSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_PairingList: components["schemas"]["CommandSuccess_PairingList"] | components["schemas"]["CommandFailure"];
        CommandResponse_PairingSnapshot: components["schemas"]["CommandSuccess_PairingSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_SearchResults: components["schemas"]["CommandSuccess_SearchResults"] | components["schemas"]["CommandFailure"];
        CommandResponse_SpaceArchive: components["schemas"]["CommandSuccess_SpaceArchive"] | components["schemas"]["CommandFailure"];
        CommandResponse_SpaceList: components["schemas"]["CommandSuccess_SpaceList"] | components["schemas"]["CommandFailure"];
        CommandResponse_SpaceSnapshot: components["schemas"]["CommandSuccess_SpaceSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_SyncCatalog: components["schemas"]["CommandSuccess_SyncCatalog"] | components["schemas"]["CommandFailure"];
        CommandResponse_SyncControl: components["schemas"]["CommandSuccess_SyncControl"] | components["schemas"]["CommandFailure"];
        CommandResponse_SyncPage: components["schemas"]["CommandSuccess_SyncPage"] | components["schemas"]["CommandFailure"];
        CommandResponse_TagList: components["schemas"]["CommandSuccess_TagList"] | components["schemas"]["CommandFailure"];
        CommandResponse_TagSnapshot: components["schemas"]["CommandSuccess_TagSnapshot"] | components["schemas"]["CommandFailure"];
        CommandResponse_TrashPreview: components["schemas"]["CommandSuccess_TrashPreview"] | components["schemas"]["CommandFailure"];
        CommandResponse_Value: components["schemas"]["CommandSuccess_Value"] | components["schemas"]["CommandFailure"];
        CommandSuccess_AccessList: {
            replayed: boolean;
            result: {
                entries: components["schemas"]["AccessSnapshot"][];
                next?: string | null;
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_AccessSnapshot: {
            replayed: boolean;
            result: {
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
        CommandSuccess_CardPlacements: {
            replayed: boolean;
            result: {
                card: components["schemas"]["CardSnapshot"];
                choices: components["schemas"]["PlacementChoice"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_CardSnapshot: {
            replayed: boolean;
            result: {
                card: string;
                columns: string[];
                completed: boolean[];
                deletion_conflict?: boolean;
                deletion_groups: string[];
                descriptions: string[];
                placements?: components["schemas"]["CardPlacement"][];
                positions: components["schemas"]["Rank"][];
                space: string;
                state: components["schemas"]["SpaceState"];
                tags: string[];
                titles: string[];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_CardTags: {
            replayed: boolean;
            result: {
                assignments: components["schemas"]["TagAssignment"][];
                card: components["schemas"]["CardSnapshot"];
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
        CommandSuccess_ColumnPositions: {
            replayed: boolean;
            result: {
                choices: components["schemas"]["ColumnPositionChoice"][];
                column: components["schemas"]["ColumnSnapshot"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_ColumnSnapshot: {
            replayed: boolean;
            result: {
                column: string;
                contents?: components["schemas"]["CardSnapshot"][] | null;
                deletion_groups: string[];
                empty: boolean;
                names: string[];
                positions: components["schemas"]["Rank"][];
                space: string;
                state: components["schemas"]["SpaceState"];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_HistoryPreview: {
            replayed: boolean;
            result: {
                card?: string | null;
                entries: components["schemas"]["HistoryEntry"][];
                removable_bytes: number;
                space: string;
                text_bytes: number;
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_ImportedSpace: {
            replayed: boolean;
            result: {
                space: string;
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_InvitationList: {
            replayed: boolean;
            result: {
                entries: components["schemas"]["InvitationSnapshot"][];
                next?: string | null;
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_InvitationSnapshot: {
            replayed: boolean;
            result: {
                created_at_unix_ms: number;
                expires_at_unix_ms: number;
                invitation: string;
                status: components["schemas"]["InvitationStatus"];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_PairingList: {
            replayed: boolean;
            result: {
                entries: components["schemas"]["PairingSnapshot"][];
                next?: string | null;
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_PairingSnapshot: {
            replayed: boolean;
            result: {
                created_at_unix_ms: number;
                invitation: string;
                label: string;
                request: string;
                status: components["schemas"]["PairingStatus"];
                subject?: string | null;
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SearchResults: {
            replayed: boolean;
            result: {
                spaces: components["schemas"]["SearchSpace"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SpaceArchive: {
            replayed: boolean;
            result: {
                archive: string;
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
                contents?: components["schemas"]["SpaceContents"] | null;
                deletion_groups: string[];
                empty: boolean;
                names: string[];
                space: string;
                state: components["schemas"]["SpaceState"];
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SyncCatalog: {
            replayed: boolean;
            result: {
                format: number;
                pairing: string;
                protocol: number;
                spaces: components["schemas"]["SyncEntry"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SyncControl: {
            replayed: boolean;
            result: {
                pairing: string;
                pause_version: components["schemas"]["StateVersion"];
                paused: boolean;
                selected: boolean;
                selection_version: components["schemas"]["StateVersion"];
                space: string;
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_SyncPage: {
            replayed: boolean;
            result: {
                base: number[][];
                changes: string[];
                complete: boolean;
                heads: number[][];
                protocol: number;
                space: string;
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_TagList: {
            replayed: boolean;
            result: {
                tags: components["schemas"]["TagSnapshot"][];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_TagSnapshot: {
            replayed: boolean;
            result: {
                deletion_conflict?: boolean;
                name_group?: string[];
                names: string[];
                space: string;
                state: components["schemas"]["TagState"];
                tag: string;
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_TrashPreview: {
            replayed: boolean;
            result: {
                cards: components["schemas"]["CardSnapshot"][];
                columns: components["schemas"]["ColumnSnapshot"][];
                conflicts: string[];
                includes_space: boolean;
                selection: components["schemas"]["TrashSelection"];
                space: string;
                version: components["schemas"]["StateVersion"];
            };
            status: components["schemas"]["SuccessStatus"];
        };
        CommandSuccess_Value: {
            replayed: boolean;
            result: unknown;
            status: components["schemas"]["SuccessStatus"];
        };
        ConfirmPairing: {
            expected: components["schemas"]["StateVersion"];
            expires_at_unix_ms?: number | null;
            request: string;
            scope: components["schemas"]["AccessScope"];
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
        CreateInvitation: {
            invitation: string;
            verifier: components["schemas"]["InvitationVerifier"];
        };
        CreateSpace: {
            name: string;
            space: string;
        };
        CreateTag: {
            name: string;
            space: string;
            tag: string;
        };
        DeleteTag: {
            expected: components["schemas"]["StateVersion"];
            space: string;
            tag: string;
        };
        EditCard: {
            card: string;
            description: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
            title: string;
        };
        ErrorCode: "invalid_request" | "incompatible_sync" | "access_denied" | "not_found" | "stale_state" | "conflict" | "command_mismatch" | "unavailable" | "outcome_unknown";
        ErrorStatus: "error";
        ExportSpace: {
            space: string;
        };
        FinishCardEditing: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            session: string;
            space: string;
        };
        HistoryEntry: {
            card: string;
            retained: boolean;
            version: components["schemas"]["CardTextVersion"];
        };
        HistoryPreview: {
            card?: string | null;
            entries: components["schemas"]["HistoryEntry"][];
            removable_bytes: number;
            space: string;
            text_bytes: number;
            version: components["schemas"]["StateVersion"];
        };
        ImportSpace: {
            archive: string;
        };
        ImportedSpace: {
            space: string;
        };
        InspectAccess: {
            subject: string;
        };
        InspectCardPlacements: {
            card: string;
            space: string;
        };
        InspectCardTags: {
            card: string;
            space: string;
        };
        InspectColumnPositions: {
            column: string;
            space: string;
        };
        InspectHistory: {
            card?: string | null;
            space: string;
        };
        InspectInvitation: {
            invitation: string;
        };
        InspectPairing: {
            request: string;
        };
        InspectSync: {
            pairing: string;
            space: string;
        };
        InspectTrash: {
            selection: components["schemas"]["TrashSelection"];
            space: string;
        };
        InvitationList: {
            entries: components["schemas"]["InvitationSnapshot"][];
            next?: string | null;
        };
        InvitationSnapshot: {
            created_at_unix_ms: number;
            expires_at_unix_ms: number;
            invitation: string;
            status: components["schemas"]["InvitationStatus"];
            version: components["schemas"]["StateVersion"];
        };
        InvitationStatus: "open" | "expired" | "cancelled" | "used" | "locked";
        InvitationVerifier: number[];
        IpcInvocation: {
            operation: string;
            request: string;
        };
        IssueAccess: {
            expires_at_unix_ms?: number | null;
            scope: components["schemas"]["AccessScope"];
            subject: string;
            verifier: components["schemas"]["KeyVerifier"];
        };
        KeyVerifier: number[];
        ListAccess: {
            after?: string | null;
        };
        ListCards: {
            include_trashed: boolean;
            space: string;
        };
        ListColumns: {
            include_trashed: boolean;
            space: string;
        };
        ListInvitations: {
            after?: string | null;
        };
        ListPairings: {
            after?: string | null;
        };
        ListSpaces: {
            include_trashed: boolean;
        };
        ListTags: {
            include_deleted: boolean;
            space: string;
        };
        MoveCard: {
            before?: string | null;
            card: string;
            column: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        MoveColumn: {
            before?: string | null;
            column: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        PairingList: {
            entries: components["schemas"]["PairingSnapshot"][];
            next?: string | null;
        };
        PairingSnapshot: {
            created_at_unix_ms: number;
            invitation: string;
            label: string;
            request: string;
            status: components["schemas"]["PairingStatus"];
            subject?: string | null;
            version: components["schemas"]["StateVersion"];
        };
        PairingStatus: "pending" | "cancelled" | "rejected" | "confirmed" | "unlinked";
        PauseSync: {
            expected: components["schemas"]["StateVersion"];
            pairing: string;
            paused: boolean;
            space: string;
        };
        PlacementChoice: {
            before?: string | null;
            placement: components["schemas"]["CardPlacement"];
        };
        PullSync: {
            known: number[][];
            protocol: number;
            space: string;
        };
        PurgeTrash: {
            expected: components["schemas"]["StateVersion"];
            selection: components["schemas"]["TrashSelection"];
            space: string;
        };
        PushSync: {
            base: number[][];
            changes: string[];
            protocol: number;
            space: string;
        };
        Rank: number | components["schemas"]["RankPart"][];
        RankPart: {
            digit: number;
            id: string;
        };
        ReadCard: {
            card: string;
            space: string;
        };
        ReadCardHistory: {
            after?: string | null;
            card: string;
            expected?: components["schemas"]["StateVersion"] | null;
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
        ReadTag: {
            space: string;
            tag: string;
        };
        RejectPairing: {
            expected: components["schemas"]["StateVersion"];
            request: string;
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
        RenameTag: {
            expected: components["schemas"]["StateVersion"];
            name: string;
            space: string;
            tag: string;
        };
        ReplaceAccess: {
            expected: components["schemas"]["StateVersion"];
            expires_at_unix_ms?: number | null;
            scope: components["schemas"]["AccessScope"];
            subject: string;
            verifier: components["schemas"]["KeyVerifier"];
        };
        RequestPairing: {
            access_verifier: components["schemas"]["KeyVerifier"];
            invitation: string;
            label: string;
            request: string;
        };
        ResolveCardDeletion: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            keep: boolean;
            space: string;
        };
        ResolveCardPlacement: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            placement: components["schemas"]["CardPlacement"];
            space: string;
        };
        ResolveCardTag: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            expected_tag: components["schemas"]["StateVersion"];
            present: boolean;
            space: string;
            tag: string;
        };
        ResolveCardText: {
            card: string;
            description: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
            title: string;
        };
        ResolveColumnDeletion: {
            column: string;
            expected: components["schemas"]["StateVersion"];
            keep: boolean;
            space: string;
        };
        ResolveColumnName: {
            column: string;
            expected: components["schemas"]["StateVersion"];
            name: string;
            space: string;
        };
        ResolveColumnPosition: {
            column: string;
            expected: components["schemas"]["StateVersion"];
            position: components["schemas"]["Rank"];
            space: string;
        };
        ResolvePurge: {
            expected: components["schemas"]["StateVersion"];
            keep: boolean;
            selection: components["schemas"]["TrashSelection"];
            space: string;
        };
        ResolveSpaceDeletion: {
            expected: components["schemas"]["StateVersion"];
            keep: boolean;
            space: string;
        };
        ResolveSpaceName: {
            expected: components["schemas"]["StateVersion"];
            name: string;
            space: string;
        };
        ResolveTagDeletion: {
            expected: components["schemas"]["StateVersion"];
            keep: boolean;
            space: string;
            tag: string;
        };
        ResolveTagNames: {
            names: components["schemas"]["TagNameChoice"][];
            space: string;
            tag: string;
        };
        RestoreCard: {
            card: string;
            column?: string | null;
            expected: components["schemas"]["StateVersion"];
            expected_column?: components["schemas"]["StateVersion"] | null;
            space: string;
        };
        RestoreCardText: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            space: string;
            text_version: string;
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
        SearchAllKind: "all";
        SearchAllScope: {
            kind: components["schemas"]["SearchAllKind"];
        };
        SearchCards: {
            query: string;
            scope: components["schemas"]["SearchScope"];
        };
        SearchHit: {
            card: components["schemas"]["CardSnapshot"];
            placement_conflict: boolean;
        };
        SearchResults: {
            spaces: components["schemas"]["SearchSpace"][];
        };
        SearchScope: components["schemas"]["SearchSpaceScope"] | components["schemas"]["SearchAllScope"];
        SearchSpace: {
            hits: components["schemas"]["SearchHit"][];
            space: components["schemas"]["SpaceSnapshot"];
        };
        SearchSpaceKind: "space";
        SearchSpaceScope: {
            kind: components["schemas"]["SearchSpaceKind"];
            space: string;
        };
        SelectSync: {
            expected: components["schemas"]["StateVersion"];
            pairing: string;
            selected: boolean;
            space: string;
        };
        SetCardCompleted: {
            card: string;
            completed: boolean;
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        SetCardTag: {
            card: string;
            expected: components["schemas"]["StateVersion"];
            expected_tag: components["schemas"]["StateVersion"];
            present: boolean;
            space: string;
            tag: string;
        };
        SpaceArchive: {
            archive: string;
        };
        SpaceContents: {
            cards: components["schemas"]["CardSnapshot"][];
            columns: components["schemas"]["ColumnSnapshot"][];
            tags: components["schemas"]["TagSnapshot"][];
        };
        SpaceList: {
            spaces: components["schemas"]["SpaceSnapshot"][];
        };
        SpaceSnapshot: {
            contents?: components["schemas"]["SpaceContents"] | null;
            deletion_groups: string[];
            empty: boolean;
            names: string[];
            space: string;
            state: components["schemas"]["SpaceState"];
            version: components["schemas"]["StateVersion"];
        };
        SpaceState: "active" | "trashed" | "conflicted";
        SpacesKind: "spaces";
        SpacesScope: {
            kind: components["schemas"]["SpacesKind"];
            spaces: {
                [key: string]: components["schemas"]["AccessMode"];
            };
        };
        StateVersion: number[];
        SuccessStatus: "ok";
        SyncCatalog: {
            format: number;
            pairing: string;
            protocol: number;
            spaces: components["schemas"]["SyncEntry"][];
        };
        SyncControl: {
            pairing: string;
            pause_version: components["schemas"]["StateVersion"];
            paused: boolean;
            selected: boolean;
            selection_version: components["schemas"]["StateVersion"];
            space: string;
        };
        SyncEntry: {
            names: string[];
            paused: boolean;
            selected: boolean;
            space: string;
            writable: boolean;
        };
        SyncPage: {
            base: number[][];
            changes: string[];
            complete: boolean;
            heads: number[][];
            protocol: number;
            space: string;
        };
        TagAssignment: {
            tag: string;
            values: boolean[];
        };
        TagList: {
            tags: components["schemas"]["TagSnapshot"][];
        };
        TagNameChoice: {
            expected: components["schemas"]["StateVersion"];
            name: string;
            tag: string;
        };
        TagSnapshot: {
            deletion_conflict?: boolean;
            name_group?: string[];
            names: string[];
            space: string;
            state: components["schemas"]["TagState"];
            tag: string;
            version: components["schemas"]["StateVersion"];
        };
        TagState: "active" | "deleted" | "conflicted";
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
        TrashPreview: {
            cards: components["schemas"]["CardSnapshot"][];
            columns: components["schemas"]["ColumnSnapshot"][];
            conflicts: string[];
            includes_space: boolean;
            selection: components["schemas"]["TrashSelection"];
            space: string;
            version: components["schemas"]["StateVersion"];
        };
        TrashSelection: {
            kind: "all";
        } | {
            kind: "space";
        } | {
            cards: string[];
            columns: string[];
            kind: "objects";
        };
        TrashSpace: {
            expected: components["schemas"]["StateVersion"];
            space: string;
        };
        UnlinkPairing: {
            expected: components["schemas"]["StateVersion"];
            request: string;
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
    issue_access: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_IssueAccess"];
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
    list_access: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListAccess"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_AccessList"];
                };
            };
        };
    };
    replace_access: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ReplaceAccess"];
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
    card_autosave: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_AutosaveCard"];
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
    card_begin_editing: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_BeginCardEditing"];
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
    card_finish_editing: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_FinishCardEditing"];
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
    card_inspect_placements: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectCardPlacements"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardPlacements"];
                };
            };
        };
    };
    card_inspect_tags: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectCardTags"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_CardTags"];
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
    card_move: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_MoveCard"];
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
    card_resolve_deletion: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveCardDeletion"];
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
    card_resolve_placement: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveCardPlacement"];
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
    card_resolve_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveCardTag"];
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
    card_resolve_text: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveCardText"];
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
    card_restore_text: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RestoreCardText"];
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
    card_search: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_SearchCards"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SearchResults"];
                };
            };
        };
    };
    card_set_completed: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_SetCardCompleted"];
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
    card_set_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_SetCardTag"];
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
    column_inspect_positions: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectColumnPositions"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ColumnPositions"];
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
    column_move: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_MoveColumn"];
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
    column_resolve_deletion: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveColumnDeletion"];
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
    column_resolve_name: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveColumnName"];
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
    column_resolve_position: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveColumnPosition"];
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
    history_clear: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ClearHistory"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_HistoryPreview"];
                };
            };
        };
    };
    history_inspect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectHistory"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_HistoryPreview"];
                };
            };
        };
    };
    invitation_cancel: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CancelInvitation"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_InvitationSnapshot"];
                };
            };
        };
    };
    invitation_create: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CreateInvitation"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_InvitationSnapshot"];
                };
            };
        };
    };
    invitation_inspect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectInvitation"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_InvitationSnapshot"];
                };
            };
        };
    };
    invitation_list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListInvitations"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_InvitationList"];
                };
            };
        };
    };
    pairing_cancel: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CancelPairing"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_PairingSnapshot"];
                };
            };
        };
    };
    pairing_confirm: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ConfirmPairing"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_PairingSnapshot"];
                };
            };
        };
    };
    pairing_inspect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectPairing"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_PairingSnapshot"];
                };
            };
        };
    };
    pairing_list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListPairings"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_PairingList"];
                };
            };
        };
    };
    pairing_reject: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RejectPairing"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_PairingSnapshot"];
                };
            };
        };
    };
    pairing_unlink: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_UnlinkPairing"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_PairingSnapshot"];
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
    space_export: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ExportSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SpaceArchive"];
                };
            };
        };
    };
    space_import: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ImportSpace"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_ImportedSpace"];
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
    space_resolve_deletion: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveSpaceDeletion"];
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
    space_resolve_name: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveSpaceName"];
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
    sync_catalog: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CatalogSync"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SyncCatalog"];
                };
            };
        };
    };
    sync_inspect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectSync"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SyncControl"];
                };
            };
        };
    };
    sync_pause: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_PauseSync"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SyncControl"];
                };
            };
        };
    };
    sync_pull: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_PullSync"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SyncPage"];
                };
            };
        };
    };
    sync_push: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_PushSync"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SyncPage"];
                };
            };
        };
    };
    sync_select: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_SelectSync"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_SyncControl"];
                };
            };
        };
    };
    tag_create: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_CreateTag"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagSnapshot"];
                };
            };
        };
    };
    tag_delete: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_DeleteTag"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagSnapshot"];
                };
            };
        };
    };
    tag_list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ListTags"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagList"];
                };
            };
        };
    };
    tag_read: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ReadTag"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagSnapshot"];
                };
            };
        };
    };
    tag_rename: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_RenameTag"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagSnapshot"];
                };
            };
        };
    };
    tag_resolve_deletion: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveTagDeletion"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagSnapshot"];
                };
            };
        };
    };
    tag_resolve_names: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolveTagNames"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TagList"];
                };
            };
        };
    };
    trash_inspect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_InspectTrash"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TrashPreview"];
                };
            };
        };
    };
    trash_purge: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_PurgeTrash"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TrashPreview"];
                };
            };
        };
    };
    trash_resolve_purge: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CommandRequest_ResolvePurge"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CommandResponse_TrashPreview"];
                };
            };
        };
    };
}
