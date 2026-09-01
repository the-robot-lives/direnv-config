# bash completion for dc (direnv-config).
#
# Install (either works):
#   1. Copy to ${XDG_DATA_HOME:-~/.local/share}/bash-completion/completions/dc
#      (done by `make install-completions`; auto-loaded by bash-completion v2).
#   2. Source this file from .bashrc.

# Config names from dc __complete-purge (one per line, designed for completion).
__dc_configs() {
    dc __complete-purge 2>/dev/null
}

# Secret key names from dc secrets (text output: indented keys under config headers).
__dc_secret_keys() {
    dc secrets 2>/dev/null | awk '/^  [^ ]/{print $1}'
}

_dc() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    COMPREPLY=()

    local commands="yaml get gen encrypt decrypt set unset prune env bump init status list purge secrets bat config compare push gen-secrets infisical"
    local global_flags="-h --help -V --version"

    # Flags whose value is the next word.
    case "$prev" in
        --layer|--replace-key|--override|--fallback|--auto|--default|\
        --value|--from|--above|--below|--note|--filter-key|--filter-value|\
        --exclude-key|--exclude-value|--to|--file|--from-envrc)
            case "$prev" in
                --from|--file|--from-envrc) COMPREPLY=($(compgen -f -- "$cur")) ;;
            esac
            return ;;
        --tier)
            COMPREPLY=($(compgen -W "0 1 2 3" -- "$cur"))
            return ;;
    esac

    # Locate the subcommand (first non-flag word after dc).
    local cmd="" cmd_i=0 i w
    for ((i = 1; i < COMP_CWORD; i++)); do
        w="${COMP_WORDS[i]}"
        case "$w" in
            -*) continue ;;
            *) cmd="$w"; cmd_i=$i; break ;;
        esac
    done

    if [[ -z "$cmd" ]]; then
        COMPREPLY=($(compgen -W "$commands $global_flags" -- "$cur"))
        return
    fi

    # Count positional args between the subcommand and the cursor.
    local pos=0
    for ((i = cmd_i + 1; i < COMP_CWORD; i++)); do
        w="${COMP_WORDS[i]}"
        case "$w" in
            --layer|--replace-key|--override|--fallback|--auto|--default|\
            --value|--from|--above|--below|--note|--filter-key|--filter-value|\
            --exclude-key|--exclude-value|--to|--file|--from-envrc|--tier)
                ((i++)) ;;
            -*) ;;
            *) ((pos++)) ;;
        esac
    done

    local opts=""
    case "$cmd" in
        yaml)
            opts="--layer --replace --replace-key --if-missing --no-bump"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        get)
            opts="--raw --override --fallback --auto --default --reveal --clippy --reveal-restricted"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        gen)
            opts="-x --hex -b --base64 --raw -t --tier"
            if [[ "$cur" != -* ]]; then
                return  # length positional — no completion
            fi ;;
        encrypt)
            opts="--value --from --stdin -t --tier --parsed --inline"
            if [[ "$cur" != -* ]]; then
                return
            fi ;;
        decrypt)
            return ;;  # token positional — no completion
        set)
            opts="--layer --no-bump"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        unset)
            opts="--layer --no-bump"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        prune)
            opts="--layer --no-bump"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        env)
            opts="--list --diff" ;;
        bump)
            return ;;
        init)
            opts="--from-envrc" ;;
        status)
            return ;;
        list)
            return ;;
        purge)
            opts="--hard"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        secrets)
            opts="--json" ;;
        bat)
            opts="--all --reveal --flat --filter-key --filter-value --exclude-key --exclude-value"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        config)
            if [[ "$cur" != -* ]]; then
                if ((pos == 0)); then
                    COMPREPLY=($(compgen -W "get set setall secure" -- "$cur"))
                    return
                else
                    local sub="${COMP_WORDS[cmd_i+1]}"
                    case "$sub" in
                        get|set|secure)
                            ((pos == 1)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                            return ;;
                        setall)
                            ((pos == 1)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                            return ;;
                    esac
                    # Fall through to flag completion for known subcommands.
                    case "$sub" in
                        set)    opts="--value --from --stdin --encrypted --yes" ;;
                        setall) opts="--above --below --yes" ;;
                        secure) opts="--note" ;;
                    esac
                fi
            fi ;;
        compare)
            opts="--to"
            if [[ "$cur" != -* ]]; then
                return  # layer, path positional — no completion
            fi ;;
        push)
            opts="--to --dry-run --yes"
            if [[ "$cur" != -* ]]; then
                ((pos == 0)) && COMPREPLY=($(compgen -W "$(__dc_configs)" -- "$cur"))
                return
            fi ;;
        gen-secrets)
            opts="--file --stdin --inline -t --tier"
            if [[ "$cur" != -* ]]; then
                return
            fi ;;
        infisical)
            if [[ "$cur" != -* ]]; then
                if ((pos == 0)); then
                    COMPREPLY=($(compgen -W "compare get set" -- "$cur"))
                    return
                else
                    local sub="${COMP_WORDS[cmd_i+1]}"
                    case "$sub" in
                        compare|get|set)
                            return ;;  # positional — no completion
                    esac
                fi
                return
            fi
            case "${COMP_WORDS[cmd_i+1]}" in
                get) opts="--reveal" ;;
                set) opts="--value --from --stdin --encrypted --yes" ;;
            esac ;;
    esac

    if [[ "$cur" == -* && -n "$opts" ]]; then
        COMPREPLY=($(compgen -W "$opts" -- "$cur"))
    fi
}

complete -F _dc dc
