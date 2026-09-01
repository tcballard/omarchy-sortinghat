#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT
package_root="$test_root/package"
test_home="$test_root/home"
fake_bin="$test_root/fake-bin"
mkdir -p "$package_root/bin" "$package_root/systemd" "$test_home" "$fake_bin"

printf '#!/usr/bin/env bash\nprintf sortinghat\n' > "$package_root/bin/sortinghat"
printf '#!/usr/bin/env bash\nprintf sortinghatd\n' > "$package_root/bin/sortinghatd"
chmod 755 "$package_root/bin/sortinghat" "$package_root/bin/sortinghatd"
cp "$repo_root/packaging/systemd/sortinghat.service" "$package_root/systemd/"
cp "$repo_root/scripts/install-package.sh" "$package_root/install.sh"
cp "$repo_root/scripts/uninstall-user.sh" "$package_root/uninstall-user.sh"
chmod 755 "$package_root/install.sh" "$package_root/uninstall-user.sh"
printf '%040d\n' 1 > "$package_root/SOURCE_COMMIT"

printf '#!/usr/bin/env bash\nexit 0\n' > "$fake_bin/systemctl"
chmod 755 "$fake_bin/systemctl"

export HOME="$test_home"
export XDG_CONFIG_HOME="$test_home/config"
export XDG_DATA_HOME="$test_home/data"
export XDG_STATE_HOME="$test_home/state"
export PATH="$fake_bin:$PATH"

"$package_root/install.sh" >/dev/null
test -x "$HOME/.local/bin/sortinghat"
test -x "$HOME/.local/bin/sortinghatd"
test -f "$XDG_CONFIG_HOME/systemd/user/sortinghat.service"
test -f "$XDG_DATA_HOME/sortinghat/provenance"

printf 'tampered\n' > "$HOME/.local/bin/sortinghat"
if "$package_root/install.sh" >/dev/null 2>&1; then
  printf 'installer overwrote a modified runtime\n' >&2
  exit 1
fi
if "$XDG_DATA_HOME/sortinghat/uninstall-user.sh" >/dev/null 2>&1; then
  printf 'uninstaller removed a modified runtime\n' >&2
  exit 1
fi

cp "$package_root/bin/sortinghat" "$HOME/.local/bin/sortinghat"
mkdir -p "$XDG_STATE_HOME/sortinghat"
printf 'preserve me\n' > "$XDG_STATE_HOME/sortinghat/state.json"
"$XDG_DATA_HOME/sortinghat/uninstall-user.sh" >/dev/null

test ! -e "$HOME/.local/bin/sortinghat"
test ! -e "$HOME/.local/bin/sortinghatd"
test ! -e "$XDG_CONFIG_HOME/systemd/user/sortinghat.service"
test ! -e "$XDG_DATA_HOME/sortinghat/uninstall-user.sh"
test -f "$XDG_STATE_HOME/sortinghat/state.json"
find "$XDG_STATE_HOME/sortinghat-uninstall-backup" -type f -name sortinghat -print -quit \
  | grep -q .
