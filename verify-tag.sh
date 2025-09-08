#!/bin/bash
set -e

DESIRED_TAG="$1"

if test -z "$DESIRED_TAG"; then
	echo "specify a version as the first argument, for example 0.1.0"
	exit 1
fi

function check_crate {
	set -e

	BASE_DIR="$1"
	CRATE="$2"
	VERSION="$3"
	CARGO_FILE="$BASE_DIR/$CRATE/Cargo.toml"

	grep -qP "^version\s*=\s*\"$VERSION\"$" "$CARGO_FILE" || return 1
	DEPS=$(grep -oP '^wow-alchemy-[\w-]+' "$CARGO_FILE")

	for DEP in $DEPS; do
		grep -qP "^$DEP\s*=.*?version\s*=\s*\"$VERSION\"" "$CARGO_FILE" || return 1
	done

	echo -n "pass"
}

echo "checking crate tags for $DESIRED_TAG..."

function check_crates {
	BASE="$1"
	CRATES="$2"
	echo "checking $BASE:"
	for NAME in $CRATES; do
		CRATE_NAME="wow-alchemy-$NAME"
		echo "- $CRATE_NAME: $(check_crate "$BASE" "$CRATE_NAME" "$DESIRED_TAG" || echo -n FAIL)"
	done
}

function check_all {
	echo "checking wow-alchemy-utils: $(check_crate "." "wow-alchemy-utils" "$DESIRED_TAG" || echo -n FAIL)"
	check_crates "file-formats" "data data-derive"
	check_crates "file-formats/database" "cdbc"
	check_crates "file-formats/graphics" "blp m2 wmo"
	check_crates "file-formats/world-data" "adt wdl wdt"
	echo "checking wow-alchemy: $(check_crate "." "wow-alchemy" "$DESIRED_TAG" || echo -n FAIL)"
	echo "checking CHANGELOG: $(grep -qP "## \[$(echo "$DESIRED_TAG" | sed 's/\./\\./g')\] - " CHANGELOG.md && echo -n pass || echo -n FAIL)"
}

OUT=$(check_all)
IFS=$'\n'
for LINE in $OUT; do
	echo $LINE
done

echo "$OUT" | grep -q FAIL && exit 1 || exit 0
