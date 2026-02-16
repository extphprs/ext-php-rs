# Changelog

## [0.1.18](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.17...cargo-php-v0.1.18) - 2026-02-16

### Added
- *(stubs)* Proper phpdoc-style comments in stubs #369 ([#676](https://github.com/extphprs/ext-php-rs/pull/676)) (by @kakserpom) [[#369](https://github.com/extphprs/ext-php-rs/issues/369)] [[#676](https://github.com/extphprs/ext-php-rs/issues/676)] 
- Eval PHP code from files ([#671](https://github.com/extphprs/ext-php-rs/pull/671)) (by @ptondereau) [[#671](https://github.com/extphprs/ext-php-rs/issues/671)] 

### Other
- *(cargo-php)* Add tests and generate deterministic output ([#677](https://github.com/extphprs/ext-php-rs/pull/677)) (by @ptondereau) [[#677](https://github.com/extphprs/ext-php-rs/issues/677)] 
## [0.1.17](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.16...cargo-php-v0.1.17) - 2026-02-05

### Fixed
- *(stubs)* Proper stub generation for interfaces ([#662](https://github.com/extphprs/ext-php-rs/pull/662)) (by @kakserpom) [[#662](https://github.com/extphprs/ext-php-rs/issues/662)] 
## [0.1.16](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.15...cargo-php-v0.1.16) - 2026-01-26

### Fixed
- *(cargo-php)* Use runtime feature for cargo-php to avoid dynamic linking on musl ([#645](https://github.com/extphprs/ext-php-rs/pull/645)) (by @ptondereau) [[#645](https://github.com/davidcole1340/ext-php-rs/issues/645)] 
## [0.1.15](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.14...cargo-php-v0.1.15) - 2025-12-28

### Added
- *(cargo-php)* Atomic extension installation and smoke testing (by @kakserpom) [[#619](https://github.com/davidcole1340/ext-php-rs/issues/619)] [[#518](https://github.com/davidcole1340/ext-php-rs/issues/518)] 
## [0.1.14](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.13...cargo-php-v0.1.14) - 2025-12-06

### Other
- *(deps)* Update libloading requirement from 0.8 to 0.9 (by @dependabot[bot])
- *(rust)* Bump Rust edition to 2024 (by @ptondereau)
## [0.1.13](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.12...cargo-php-v0.1.13) - 2025-10-29

### Other
- Change links for org move (by @Xenira) [[#500](https://github.com/davidcole1340/ext-php-rs/issues/500)] 
## [0.1.12](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.11...cargo-php-v0.1.12) - 2025-10-28

### Added
- *(enum)* Add basic enum support (by @Xenira, @joehoyle) [[#178](https://github.com/extphprs/ext-php-rs/issues/178)] [[#302](https://github.com/extphprs/ext-php-rs/issues/302)] 

### Other
- *(clippy)* Fix new clippy errors (by @Xenira) [[#558](https://github.com/extphprs/ext-php-rs/issues/558)] 
- *(deps)* Update cargo_metadata requirement from 0.22 to 0.23 (by @dependabot[bot])
- *(deps)* Update dialoguer requirement from 0.11 to 0.12 (by @dependabot[bot])
- *(deps)* Update cargo_metadata requirement from 0.21 to 0.22 (by @dependabot[bot])
- *(deps)* Update cargo_metadata requirement from 0.20 to 0.21 (by @dependabot[bot])
- Update guide url and authors (by @Xenira) [[#500](https://github.com/extphprs/ext-php-rs/issues/500)] 
## [0.1.11](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.10...cargo-php-v0.1.11) - 2025-07-04

### Added
- *(cargo-php)* --features, --all-features, --no-default-features (by @kakserpom)

### Fixed
- *(cargo-php)* `get_ext_dir()`/`get_php_ini()` stdout noise tolerance (by @kakserpom) [[#459](https://github.com/extphprs/ext-php-rs/issues/459)] 
- *(clippy)* Fix new clippy findings (by @Xenira)

### Other
- *(cargo-php)* Add locked option to install guide ([#370](https://github.com/extphprs/ext-php-rs/pull/370)) (by @Xenira) [[#370](https://github.com/extphprs/ext-php-rs/issues/370)] [[#314](https://github.com/extphprs/ext-php-rs/issues/314)] 
- *(cli)* Enforce docs for cli (by @Xenira) [[#392](https://github.com/extphprs/ext-php-rs/issues/392)] 
- *(clippy)* Apply pedantic rules (by @Xenira) [[#418](https://github.com/extphprs/ext-php-rs/issues/418)] 
- *(deps)* Update cargo_metadata requirement from 0.19 to 0.20 ([#437](https://github.com/extphprs/ext-php-rs/pull/437)) (by @dependabot[bot]) [[#437](https://github.com/extphprs/ext-php-rs/issues/437)] 
- *(deps)* Update cargo_metadata requirement from 0.15 to 0.19 ([#404](https://github.com/extphprs/ext-php-rs/pull/404)) (by @dependabot[bot]) [[#404](https://github.com/extphprs/ext-php-rs/issues/404)] 
- *(deps)* Update libloading requirement from 0.7 to 0.8 ([#389](https://github.com/extphprs/ext-php-rs/pull/389)) (by @dependabot[bot]) [[#389](https://github.com/extphprs/ext-php-rs/issues/389)] 
- *(deps)* Update dialoguer requirement from 0.10 to 0.11 ([#387](https://github.com/extphprs/ext-php-rs/pull/387)) (by @dependabot[bot]) [[#387](https://github.com/extphprs/ext-php-rs/issues/387)] 

## [0.1.10](https://github.com/extphprs/ext-php-rs/compare/cargo-php-v0.1.9...cargo-php-v0.1.10) - 2025-02-06

### Other
- *(release)* Add release bot (#346) (by @Xenira) [[#346](https://github.com/extphprs/ext-php-rs/issues/346)] [[#340](https://github.com/extphprs/ext-php-rs/issues/340)] 
- Don't use symbolic links for git. (by @faassen)
- Fix pipeline (#320) (by @Xenira) [[#320](https://github.com/extphprs/ext-php-rs/issues/320)] 
