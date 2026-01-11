#!/usr/bin/env python3
"""Remove a CLI version from cli-releases.json"""

import json
import sys
from datetime import datetime, timezone


def main():
    if len(sys.argv) != 2:
        print("Usage: unrelease-cli.py <version>", file=sys.stderr)
        sys.exit(1)

    version_to_remove = sys.argv[1]

    with open('cli-releases.json', 'r') as f:
        data = json.load(f)

    # Check if version exists
    if version_to_remove not in data.get('releases', {}):
        print(f"Error: Version {version_to_remove} not found in cli-releases.json", file=sys.stderr)
        sys.exit(1)

    # Check if we need to update latest
    if data.get('latest') == version_to_remove:
        releases = data.get('releases') or {}
        remaining = [v for v in releases.keys() if v != version_to_remove]
        if remaining:
            def version_key(v):
                try:
                    return tuple(int(x) for x in v.split('.'))
                except:
                    return (0, 0, 0)
            remaining.sort(key=version_key)
            data['latest'] = remaining[-1]
            print(f"  Updated latest to {data['latest']}")
        else:
            print("  Warning: No other versions found")

    # Remove the version
    del data['releases'][version_to_remove]
    print(f"  Removed {version_to_remove} from releases")

    # Update timestamp
    data['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')

    with open('cli-releases.json', 'w') as f:
        json.dump(data, f, indent=2)
        f.write('\n')

    print("  ✓ Manifest updated")


if __name__ == '__main__':
    main()
