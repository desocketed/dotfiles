# homun-bootstrap <dir> <rev> <url>: make sure the launcher's clone can run the host service,
# before each start of it. Afterwards the service follows the pin by itself.
#   - no clone yet (a first switch): clone it;
#   - a clone from before the host service existed: bring it to the pinned commit once, by the
#     service's own rule (no uncommitted change to a tracked file, no commit that was never
#     published); a clone that holds such work is left alone and the start fails, saying why.
set -eu
dir=$1 rev=$2 url=$3
g() { git -C "$dir" "$@"; }

if [ ! -e "$dir" ]; then exec git clone -q "$url" "$dir"; fi
if [ -e "$dir/bin/homun-host" ]; then exit 0; fi
if [ "$(g remote get-url origin 2>/dev/null)" != "$url" ]; then
	echo "homun: $dir is not a clone of $url and has no host service; update it yourself" >&2
	exit 1
fi
if [ -n "$(g status --porcelain --untracked-files=no)" ]; then
	echo "homun: $dir has uncommitted changes; not moving it to the pinned $rev" >&2
	exit 1
fi
# Published: whatever origin's refs point to or pointed to before, and where a switch left it
published=$(
	{
		g for-each-ref --format='%(refname)' refs/remotes/origin | while read -r ref; do
			g rev-parse "$ref"
			g log -g --format=%H "$ref" 2>/dev/null || true
		done
		cat "$dir/.git/homun-pin" 2>/dev/null || true
	} | sort -u | while read -r c; do
		if g cat-file -e "$c^{commit}" 2>/dev/null; then echo "$c"; fi
	done
)
# shellcheck disable=SC2086
own=$(g rev-list HEAD --not $published)
if [ -n "$own" ]; then
	echo "homun: $dir has commits that were never published; not moving it. To discard them: git -C $dir fetch origin $rev; git -C $dir reset --hard $rev" >&2
	exit 1
fi
g cat-file -e "$rev^{commit}" 2>/dev/null || g fetch -q origin "$rev" || g fetch -q origin
g reset -q --hard "$rev"
echo "$rev" > "$dir/.git/homun-pin"
echo "homun: moved $dir to the pinned $rev"
