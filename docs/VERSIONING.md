# Versioning and releases

Tags use `vMAJOR.MILESTONE.PRODUCTION.TESTING`:

- MAJOR marks a major rework or large milestone; MILESTONE marks a smaller milestone or completed feature group.
- PRODUCTION identifies the production release; TESTING is a testing revision or small fix.
- Production tags end in `.0` with no letter, for example `v1.2.1.0`. Their first three numbers must exactly match `package.version` in `Cargo.toml`.
- Testing tags end in a nonzero testing revision followed by `a` (experimental alpha) or `b` (relatively safe beta), for example `v1.2.1.4a` or `v1.2.1.5b`. They produce workflow artifacts retained for 14 days, not a GitHub Release.
- A nonzero revision without a suffix, a zero testing revision with a suffix, and any other suffix are invalid.

Cargo uses the three-part version `MAJOR.MILESTONE.PRODUCTION`; update it and `Cargo.lock` when that version changes. To release, commit the version change, create the four-part tag, and push that tag. Production tags automatically publish a GitHub Release; ordinary pushes do not.

The Windows executable version resource currently reports the fixed package version `0.1.0.0`; update `assets/app.rc` when the package version changes. Testing tag letters are not embedded in the executable.
