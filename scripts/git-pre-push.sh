#!/bin/sh
# Git supplies one ref update per line on stdin. Publish branches and tags only.
set -eu

while read -r local_ref local_oid remote_ref remote_oid; do
    case "$local_ref" in
        refs/heads/*|refs/tags/*|'(delete)') ;;
        HEAD)
            named_ref=$(git symbolic-ref --quiet HEAD) || {
                echo 'Push rejected: detached HEAD must be published through a named branch or tag.' >&2
                exit 1
            }
            case "$named_ref" in
                refs/heads/*) ;;
                *) exit 1 ;;
            esac
            test "$(git rev-parse --verify "$named_ref")" = "$local_oid" || exit 1
            ;;
        *)
            echo 'Push rejected: publish named branches or tags, not internal refs or raw objects.' >&2
            exit 1
            ;;
    esac
    case "$remote_ref" in
        refs/heads/*|refs/tags/*) ;;
        *)
            echo 'Push rejected: remote refs must be branches or tags; internal refs must stay local.' >&2
            exit 1
            ;;
    esac
done
