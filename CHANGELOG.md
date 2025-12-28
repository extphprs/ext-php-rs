# Changelog

## [0.15.3](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.15.2...ext-php-rs-v0.15.3) - 2025-12-28

### Added
- *(cargo-php)* Atomic extension installation and smoke testing (by @kakserpom) [[#619](https://github.com/davidcole1340/ext-php-rs/issues/619)] [[#518](https://github.com/davidcole1340/ext-php-rs/issues/518)] 
- *(class)* Static properties and methods #252 ([#627](https://github.com/extphprs/ext-php-rs/pull/627)) (by @kakserpom) [[#252](https://github.com/davidcole1340/ext-php-rs/issues/252)] [[#627](https://github.com/davidcole1340/ext-php-rs/issues/627)] 
- *(is_scalar)* Zval::is_scalar() #237 ([#614](https://github.com/extphprs/ext-php-rs/pull/614)) (by @kakserpom) [[#237](https://github.com/davidcole1340/ext-php-rs/issues/237)] [[#614](https://github.com/davidcole1340/ext-php-rs/issues/614)] 
- *(php_write)* A binary-safe way to write to PHP's stdout/stderr #508 ([#613](https://github.com/extphprs/ext-php-rs/pull/613)) (by @kakserpom) [[#508](https://github.com/davidcole1340/ext-php-rs/issues/508)] [[#613](https://github.com/davidcole1340/ext-php-rs/issues/613)] 
- *(types)* Argument coercion into HashSet/BTreeSet #493 ([#598](https://github.com/extphprs/ext-php-rs/pull/598)) (by @kakserpom) [[#493](https://github.com/davidcole1340/ext-php-rs/issues/493)] [[#598](https://github.com/davidcole1340/ext-php-rs/issues/598)] 
- Add support for empty immutable shared arrays (by @kakserpom) [[#631](https://github.com/davidcole1340/ext-php-rs/issues/631)] [[#355](https://github.com/davidcole1340/ext-php-rs/issues/355)] 

### Fixed
- *(binary)* Avoid leaking in Pack::pack_into ([#610](https://github.com/extphprs/ext-php-rs/pull/610)) (by @Nova-Static) [[#610](https://github.com/davidcole1340/ext-php-rs/issues/610)] 
- *(bindings)* Exclude `preserve_none` instruction from bindings ([#620](https://github.com/extphprs/ext-php-rs/pull/620)) (by @ptondereau) [[#620](https://github.com/davidcole1340/ext-php-rs/issues/620)] 
- *(ci)* Disable TS build for embed tests ([#600](https://github.com/extphprs/ext-php-rs/pull/600)) (by @ptondereau) [[#600](https://github.com/davidcole1340/ext-php-rs/issues/600)] 
- *(class)* Check property visibility (by @kakserpom) [[#633](https://github.com/davidcole1340/ext-php-rs/issues/633)] [[#375](https://github.com/davidcole1340/ext-php-rs/issues/375)] 
- *(class)* Return Self ($this) #502 ([#626](https://github.com/extphprs/ext-php-rs/pull/626)) (by @kakserpom) [[#502](https://github.com/davidcole1340/ext-php-rs/issues/502)] [[#626](https://github.com/davidcole1340/ext-php-rs/issues/626)] 
- *(clippy)* Default hasher for ArrayKey ([#608](https://github.com/extphprs/ext-php-rs/pull/608)) (by @ptondereau) [[#608](https://github.com/davidcole1340/ext-php-rs/issues/608)] 
- *(clippy)* Clippy v1.92.0 ([#605](https://github.com/extphprs/ext-php-rs/pull/605)) (by @ptondereau) [[#605](https://github.com/davidcole1340/ext-php-rs/issues/605)] 
- *(macro)* Identifier-related bugs #536 ([#616](https://github.com/extphprs/ext-php-rs/pull/616)) (by @kakserpom) [[#536](https://github.com/davidcole1340/ext-php-rs/issues/536)] [[#616](https://github.com/davidcole1340/ext-php-rs/issues/616)] 
- *(macro)* Nullable parameters #538 ([#617](https://github.com/extphprs/ext-php-rs/pull/617)) (by @kakserpom) [[#538](https://github.com/davidcole1340/ext-php-rs/issues/538)] [[#617](https://github.com/davidcole1340/ext-php-rs/issues/617)] 
- *(macro)* Reference mutability inside Option #515 ([#615](https://github.com/extphprs/ext-php-rs/pull/615)) (by @kakserpom) [[#515](https://github.com/davidcole1340/ext-php-rs/issues/515)] [[#615](https://github.com/davidcole1340/ext-php-rs/issues/615)] 
- *(zend_bailout)* Fix zend_bailout handling #537 ([#625](https://github.com/extphprs/ext-php-rs/pull/625)) (by @kakserpom) [[#537](https://github.com/davidcole1340/ext-php-rs/issues/537)] [[#625](https://github.com/davidcole1340/ext-php-rs/issues/625)] 
- *(zval)* Heap corruption with persistent=true #424 ([#622](https://github.com/extphprs/ext-php-rs/pull/622)) (by @kakserpom) [[#424](https://github.com/davidcole1340/ext-php-rs/issues/424)] [[#622](https://github.com/davidcole1340/ext-php-rs/issues/622)] 

### Other
- *(deps)* Update zip requirement from 6.0 to 7.0 ([#634](https://github.com/extphprs/ext-php-rs/pull/634)) (by @dependabot[bot]) [[#634](https://github.com/davidcole1340/ext-php-rs/issues/634)] 
- *(deps)* Bump actions/cache from 4 to 5 ([#607](https://github.com/extphprs/ext-php-rs/pull/607)) (by @dependabot[bot]) [[#607](https://github.com/davidcole1340/ext-php-rs/issues/607)] 
- *(deps)* Bump JamesIves/github-pages-deploy-action ([#606](https://github.com/extphprs/ext-php-rs/pull/606)) (by @dependabot[bot]) [[#606](https://github.com/davidcole1340/ext-php-rs/issues/606)] 
- *(superglobals)* A guide chapter on Superglobals #499 ([#612](https://github.com/extphprs/ext-php-rs/pull/612)) (by @kakserpom) [[#499](https://github.com/davidcole1340/ext-php-rs/issues/499)] [[#612](https://github.com/davidcole1340/ext-php-rs/issues/612)] 
- Remove outdated `ClassBuilder::property()` documentation ([#603](https://github.com/extphprs/ext-php-rs/pull/603)) (by @DanielEScherzer) [[#603](https://github.com/davidcole1340/ext-php-rs/issues/603)] [[#372](https://github.com/davidcole1340/ext-php-rs/issues/372)] 
## [0.15.2](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.15.1...ext-php-rs-v0.15.2) - 2025-12-06

### Added
- *(builders)* Add send_headers setter to SapiBuilder (by @Qard)
- *(interface)* Add support for interfaces (by @Norbytus) [[#533](https://github.com/davidcole1340/ext-php-rs/issues/533)] 
- *(php)* Add PHP 8.5 support ([#592](https://github.com/extphprs/ext-php-rs/pull/592)) (by @ptondereau) [[#592](https://github.com/davidcole1340/ext-php-rs/issues/592)] 

### Fixed
- *(musl)* Use Alpine 3.22 for musl tests ([#595](https://github.com/extphprs/ext-php-rs/pull/595)) (by @ptondereau) [[#595](https://github.com/davidcole1340/ext-php-rs/issues/595)] 
- *(release-plz)* Update workflow to run release-plz ([#578](https://github.com/extphprs/ext-php-rs/pull/578)) (by @ptondereau) [[#578](https://github.com/davidcole1340/ext-php-rs/issues/578)] 
- *(releaze-plz)* Trusted publishing token ([#579](https://github.com/extphprs/ext-php-rs/pull/579)) (by @ptondereau) [[#579](https://github.com/davidcole1340/ext-php-rs/issues/579)] 

### Other
- *(deps)* Update convert_case requirement from 0.9.0 to 0.10.0 ([#593](https://github.com/extphprs/ext-php-rs/pull/593)) (by @dependabot[bot]) [[#593](https://github.com/davidcole1340/ext-php-rs/issues/593)] 
- *(deps)* Bump JamesIves/github-pages-deploy-action (by @dependabot[bot])
- *(deps)* Update convert_case requirement from 0.8.0 to 0.9.0 (by @dependabot[bot])
- *(deps)* Update libloading requirement from 0.8 to 0.9 (by @dependabot[bot])
- *(docs)* Fix docs workflow permissions (by @Xenira)
- *(release)* Pass github pat to release workflow (by @Xenira)
- *(release)* Fix permissions again (by @Xenira)
- *(release)* Add missing permissions to release trigger (by @Xenira) [[#580](https://github.com/davidcole1340/ext-php-rs/issues/580)] 
- *(release)* Trigger release via reusable workflow (by @Xenira) [[#579](https://github.com/davidcole1340/ext-php-rs/issues/579)] [[#578](https://github.com/davidcole1340/ext-php-rs/issues/578)] 
- Add musl build step ([#561](https://github.com/extphprs/ext-php-rs/pull/561)) (by @ptondereau) [[#561](https://github.com/davidcole1340/ext-php-rs/issues/561)] 
- Restructure master and pr workflows (by @Xenira)
## [0.15.1](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.15.0...ext-php-rs-v0.15.1) - 2025-10-29

### Other
- *(release)* Switch to trusted publishing (by @Xenira)
- Update build warning to reflect support status of php 8.0 (by @Xenira)
- Remove tests for eol PHP 8.0 (by @Xenira)
- Change links for org move (by @Xenira) [[#500](https://github.com/davidcole1340/ext-php-rs/issues/500)] 
## [0.15.0](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.14.2...ext-php-rs-v0.15.0) - 2025-10-28

### BREAKING CHANGES

- *(stubs)* [**breaking**] Add stubs for `RustClosure` (by @Xenira) [[#373](https://github.com/extphprs/ext-php-rs/issues/373)] 
> New field `variadic` added to `Parameter` struct.
- *(enum)* [**breaking**] Add basic enum support (by @Xenira, @joehoyle) [[#178](https://github.com/extphprs/ext-php-rs/issues/178)] [[#302](https://github.com/extphprs/ext-php-rs/issues/302)] 
> New field `enums` added to `Module` struct.

### Added
- *(array)* Introducing BTreeMap conversion and refactoring HashMap conversion (by @kakserpom) [[#535](https://github.com/extphprs/ext-php-rs/issues/535)] 
- *(array)* Support `Vec<(K,V)>` for hashtables (by @Xenira) [[#425](https://github.com/extphprs/ext-php-rs/issues/425)] 
- *(module)* Add `ModuleBuilder` name and version setters (by @kakserpom) [[#534](https://github.com/extphprs/ext-php-rs/issues/534)] 
- *(zval)* Add `Zval::null()` (by @kakserpom) [[#521](https://github.com/extphprs/ext-php-rs/issues/521)] 
- Add constructor visibility (by @Norbytus) [[#542](https://github.com/extphprs/ext-php-rs/issues/542)] 

### Fixed
- *(array)* Don't convert array keys with leading zeros (by @tomterl)
- *(embed)* Add missing zend_destroy_file_handle (by @el7cosmos) [[#556](https://github.com/extphprs/ext-php-rs/issues/556)] 

### Other
- *(array)* Split `array.rs` types into smaller files (by @ptondereau) [[#524](https://github.com/extphprs/ext-php-rs/issues/524)] 
- *(clippy)* Fix new clippy errors (by @Xenira) [[#558](https://github.com/extphprs/ext-php-rs/issues/558)] 
- *(clippy)* Fix new clippy findings (by @Xenira) [[#543](https://github.com/extphprs/ext-php-rs/issues/543)] 
- *(deps)* Upgrade bindgen to 0.72 (by @ptondereau)
- *(deps)* Update zip requirement from 5.1 to 6.0 (by @dependabot[bot])
- *(deps)* Bump actions/stale from 9 to 10 (by @dependabot[bot])
- *(deps)* Update cargo_metadata requirement from 0.22 to 0.23 (by @dependabot[bot])
- *(deps)* Update zip requirement from 4.0 to 5.1 (by @dependabot[bot])
- *(deps)* Remove unused deps (by @robem)
- *(deps)* Update dialoguer requirement from 0.11 to 0.12 (by @dependabot[bot])
- *(deps)* Update cargo_metadata requirement from 0.21 to 0.22 (by @dependabot[bot])
- *(deps)* Bump actions/checkout from 4 to 5 (by @dependabot[bot])
- *(macro)* Add test infrastructure for macro crate (by @Xenira) [[#530](https://github.com/extphprs/ext-php-rs/issues/530)] 
- *(php)* Use PHP 8.4 for linting (by @ptondereau)
- *(readme)* Update example in readme (by @joehoyle) [[#539](https://github.com/extphprs/ext-php-rs/issues/539)] 
- Add ptondereau to maintainer list (by @ptondereau)
- Remove old ZTS docker and remove docker for embed tests (by @ptondereau)
- Update guide url and authors (by @Xenira) [[#500](https://github.com/extphprs/ext-php-rs/issues/500)] 
## [0.14.2](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.14.1...ext-php-rs-v0.14.2) - 2025-07-13

### Added
- Make Sapi work with ZTS builds (by @Qard) [[#488](https://github.com/extphprs/ext-php-rs/issues/488)] 

### Fixed
- *(clippy)* Fix new clippy rule (by @Xenira)
- *(macro)* Allow multiple refs with `self_` (by @Xenira) [[#506](https://github.com/extphprs/ext-php-rs/issues/506)] 
- *(macro)* Fix `self_` reference when multiple method arguments supplied (by @Xenira) [[#504](https://github.com/extphprs/ext-php-rs/issues/504)] 
- *(stubs)* Include doc comments in `__construct()` stubs (by @Xenira) [[#497](https://github.com/extphprs/ext-php-rs/issues/497)] 

### Other
- *(args)* Add missing `embed` feature guard (by @Xenira) [[#501](https://github.com/extphprs/ext-php-rs/issues/501)] 
- *(deps)* Upgrade bindgen to `v0.70` (by @Xenira)
- *(release-plz)* Only run release workflow on build and lint success (by @Xenira)

## [0.14.1](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.14.0...ext-php-rs-v0.14.1) - 2025-07-06

### Added
- *(error)* Deprecate `E_USER_ERROR` in PHP 8.4 (by @Xenira) [[#476](https://github.com/extphprs/ext-php-rs/issues/476)] 

### Fixed
- Support defaults for calls with named arguments (by @Stranger6667) [[#490](https://github.com/extphprs/ext-php-rs/issues/490)] 

### Other
- Add missing parenthesis (by @Stranger6667) [[#486](https://github.com/extphprs/ext-php-rs/issues/486)] 

## [0.14.0](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.13.1...ext-php-rs-v0.14.0) - 2025-07-04

### BREAKING CHANGES

- *(macro)* [**breaking**] Change rename defaults to match psr (by @Xenira) [[#189](https://github.com/extphprs/ext-php-rs/issues/189)] [[#436](https://github.com/extphprs/ext-php-rs/issues/436)] 
> Methods and Properties are renamed to camelCase by default. Classes to PascalCase, constants to UPPER_CASE and functions to snake_case
- *(class)* [**breaking**] Generate correct stubs for extends and implements (by @Xenira) [[#326](https://github.com/extphprs/ext-php-rs/issues/326)] 
> `extends` and `implements` attributes now require the `stub` property containing the class/interface name to be used in stubs.
- *(macro)* [**breaking**] Uinify attributes in `#[php]` attribute (by @Xenira) [[#391](https://github.com/extphprs/ext-php-rs/issues/391)] 
> Attributes like `#[prop]`, `#[rename]`, etc. have been moved to `#[php]` attributes like `#[php(prop)]`, `#[php(name = "Foo")]`, `#[php(change_case = CamelCase)]`, etc.
- *(macro)* [**breaking**] Switch to builder pattern (by @davidcole1340, @danog, @ptondereau, @Xenira) [[#99](https://github.com/extphprs/ext-php-rs/issues/99)] [[#131](https://github.com/extphprs/ext-php-rs/issues/131)] [[#327](https://github.com/extphprs/ext-php-rs/issues/327)] [[#174](https://github.com/extphprs/ext-php-rs/issues/174)] [[#335](https://github.com/extphprs/ext-php-rs/issues/335)] 
> The old macros were dependent on execution order and have been causing trouble with language servers. They are replaced by a builder. See the migration guide at https://davidcole1340.github.io/ext-php-rs/migration-guides/v0.14.html for information on how to migrate.

### Added
- *(alloc)* Add estrdup (by @Qard) [[#444](https://github.com/extphprs/ext-php-rs/issues/444)] 
- *(builders)* Add IniBuilder (by @Qard) [[#442](https://github.com/extphprs/ext-php-rs/issues/442)] 
- *(cargo-php)* --features, --all-features, --no-default-features (by @kakserpom)
- *(ffi)* Allow definging additional bindings (by @Xenira) [[#403](https://github.com/extphprs/ext-php-rs/issues/403)] 
- *(globals)* Add `CompilerGlobals` (by @Qard) [[#445](https://github.com/extphprs/ext-php-rs/issues/445)] 
- *(sapi)* Expand `SapiBuilder` (by @Qard) [[#471](https://github.com/extphprs/ext-php-rs/issues/471)] 
- *(zts)* Set lock per thread on zts build ([#408](https://github.com/extphprs/ext-php-rs/pull/408)) (by @joelwurtz) [[#408](https://github.com/extphprs/ext-php-rs/issues/408)] 
- Argument defaults can be any expr valid in const scope (by @alekitto) [[#433](https://github.com/extphprs/ext-php-rs/issues/433)] 

### Fixed
- *(FunctionBuilder)* Remove `null` type from non-nullable types `void` and `mixed` (by @kakserpom) [[#457](https://github.com/extphprs/ext-php-rs/issues/457)] 
- *(args)* Fix variadic args (by @Xenira) [[#337](https://github.com/extphprs/ext-php-rs/issues/337)] 
- *(array)* Cast numeric string keys into `zend_ulong` and allow negative keys (by @kakserpom) [[#453](https://github.com/extphprs/ext-php-rs/issues/453)] [[#454](https://github.com/extphprs/ext-php-rs/issues/454)] [[#456](https://github.com/extphprs/ext-php-rs/issues/456)] 
- *(build)* Take only the first line of which/where when searching for php executable (by @alekitto) [[#430](https://github.com/extphprs/ext-php-rs/issues/430)] 
- *(cargo-php)* `get_ext_dir()`/`get_php_ini()` stdout noise tolerance (by @kakserpom) [[#459](https://github.com/extphprs/ext-php-rs/issues/459)] 
- *(clippy)* Fix new clippy findings (by @Xenira)
- *(globals)* Split sapi header value containing `:` correctly (by @Qard) [[#441](https://github.com/extphprs/ext-php-rs/issues/441)] 
- *(macro)* Add missing static flags in `php_impl` macro (by @Norbytus) [[#419](https://github.com/extphprs/ext-php-rs/issues/419)] 
- *(macro)* Add missing separator pipe in flags (by @Norbytus) [[#412](https://github.com/extphprs/ext-php-rs/issues/412)] 
- Ensure update_bindings runs as amd64 (by @Qard) [[#446](https://github.com/extphprs/ext-php-rs/issues/446)] [[#448](https://github.com/extphprs/ext-php-rs/issues/448)] 
- Fix object access in object write/read/has_property handlers (by @alekitto) [[#313](https://github.com/extphprs/ext-php-rs/issues/313)] [[#438](https://github.com/extphprs/ext-php-rs/issues/438)] 

### Other
- *(bindings)* Update `docsrs_bindings.rs` to PHP 8.4 (by @Xenira) [[#447](https://github.com/extphprs/ext-php-rs/issues/447)] 
- *(bindings)* Add tooling to generate `docsrs_bindings.rs` (by @Xenira) [[#443](https://github.com/extphprs/ext-php-rs/issues/443)] 
- *(build)* Fix typo in `build.rs` (by @Xenira) [[#439](https://github.com/extphprs/ext-php-rs/issues/439)] 
- *(cargo-php)* Add locked option to install guide ([#370](https://github.com/extphprs/ext-php-rs/pull/370)) (by @Xenira) [[#370](https://github.com/extphprs/ext-php-rs/issues/370)] [[#314](https://github.com/extphprs/ext-php-rs/issues/314)] 
- *(cli)* Enforce docs for cli (by @Xenira) [[#392](https://github.com/extphprs/ext-php-rs/issues/392)] 
- *(clippy)* Apply pedantic rules (by @Xenira) [[#418](https://github.com/extphprs/ext-php-rs/issues/418)] 
- *(coverage)* Add coverage badge (by @Xenira)
- *(coverage)* Ignore release pr (by @Xenira)
- *(coverage)* Add coverage reporting (by @Xenira) [[#415](https://github.com/extphprs/ext-php-rs/issues/415)] 
- *(dependabot)* Remove redundant directories included in workspace ([#386](https://github.com/extphprs/ext-php-rs/pull/386)) (by @Xenira) [[#386](https://github.com/extphprs/ext-php-rs/issues/386)] 
- *(dependabot)* Add cargo ecosystem ([#378](https://github.com/extphprs/ext-php-rs/pull/378)) (by @Xenira) [[#378](https://github.com/extphprs/ext-php-rs/issues/378)] 
- *(deps)* Update cargo_metadata requirement from 0.19 to 0.20 ([#437](https://github.com/extphprs/ext-php-rs/pull/437)) (by @dependabot[bot]) [[#437](https://github.com/extphprs/ext-php-rs/issues/437)] 
- *(deps)* Update zip requirement from 3.0 to 4.0 ([#435](https://github.com/extphprs/ext-php-rs/pull/435)) (by @dependabot[bot]) [[#435](https://github.com/extphprs/ext-php-rs/issues/435)] 
- *(deps)* Update zip requirement from 2.2 to 3.0 ([#432](https://github.com/extphprs/ext-php-rs/pull/432)) (by @dependabot[bot]) [[#432](https://github.com/extphprs/ext-php-rs/issues/432)] 
- *(deps)* Update cargo_metadata requirement from 0.15 to 0.19 ([#404](https://github.com/extphprs/ext-php-rs/pull/404)) (by @dependabot[bot]) [[#404](https://github.com/extphprs/ext-php-rs/issues/404)] 
- *(deps)* Update syn and darling ([#400](https://github.com/extphprs/ext-php-rs/pull/400)) (by @Xenira) [[#400](https://github.com/extphprs/ext-php-rs/issues/400)] 
- *(deps)* Update ureq requirement from 2.4 to 3.0 ([#379](https://github.com/extphprs/ext-php-rs/pull/379)) (by @dependabot[bot]) [[#379](https://github.com/extphprs/ext-php-rs/issues/379)] 
- *(deps)* Update libloading requirement from 0.7 to 0.8 ([#389](https://github.com/extphprs/ext-php-rs/pull/389)) (by @dependabot[bot]) [[#389](https://github.com/extphprs/ext-php-rs/issues/389)] 
- *(deps)* Update dialoguer requirement from 0.10 to 0.11 ([#387](https://github.com/extphprs/ext-php-rs/pull/387)) (by @dependabot[bot]) [[#387](https://github.com/extphprs/ext-php-rs/issues/387)] 
- *(deps)* Update zip requirement from 0.6 to 2.2 ([#381](https://github.com/extphprs/ext-php-rs/pull/381)) (by @dependabot[bot]) [[#381](https://github.com/extphprs/ext-php-rs/issues/381)] 
- *(deps)* Bump JamesIves/github-pages-deploy-action ([#374](https://github.com/extphprs/ext-php-rs/pull/374)) (by @dependabot[bot]) [[#374](https://github.com/extphprs/ext-php-rs/issues/374)] 
- *(github)* Add issue and pr templates (by @Xenira) [[#455](https://github.com/extphprs/ext-php-rs/issues/455)] 
- *(guide)* Directly include doc comments (by @Xenira)
- *(hooks)* Add check for outdated macro documentation (by @Xenira) [[#466](https://github.com/extphprs/ext-php-rs/issues/466)] 
- *(integration)* Reorganise integration tests (by @Xenira) [[#414](https://github.com/extphprs/ext-php-rs/issues/414)] 
- *(macro)* Change `rename` to `change_case` (by @Xenira)
- *(macro)* Improve `name` vs `rename` documentation (by @Xenira) [[#422](https://github.com/extphprs/ext-php-rs/issues/422)] 
- *(macro)* Use `#[php]` attribute for startup function (by @Xenira) [[#423](https://github.com/extphprs/ext-php-rs/issues/423)] 
- *(macro)* Trait rename for general and method names (by @Norbytus) [[#420](https://github.com/extphprs/ext-php-rs/issues/420)] 
- *(macro)* Update documentation for builder pattern (by @Xenira)
- *(macro)* Add stubs for new builder pattern (by @Xenira) [[#183](https://github.com/extphprs/ext-php-rs/issues/183)] 
- *(php-tokio)* Move documentation into separate section (by @Xenira) [[#322](https://github.com/extphprs/ext-php-rs/issues/322)] 
- *(release-plz)* Move breaking changes to section on top of changelog ([#393](https://github.com/extphprs/ext-php-rs/pull/393)) (by @Xenira) [[#393](https://github.com/extphprs/ext-php-rs/issues/393)] 
- *(sapi)* Use builder pattern in sapi test (by @Xenira)
- *(test)* Fix embed test on php 8.4 ([#396](https://github.com/extphprs/ext-php-rs/pull/396)) (by @joelwurtz) [[#396](https://github.com/extphprs/ext-php-rs/issues/396)] 
- *(test)* Disable inline example tests for macos unstable ([#377](https://github.com/extphprs/ext-php-rs/pull/377)) (by @Xenira) [[#377](https://github.com/extphprs/ext-php-rs/issues/377)] 
- Add git hooks and `CONTRIBUTING.md` (by @Xenira) [[#475](https://github.com/extphprs/ext-php-rs/issues/475)] 
- Improve test reliability and ease of use (by @Qard) [[#450](https://github.com/extphprs/ext-php-rs/issues/450)] 
- Add missing cfg gate to anyhow exception test (by @Qard) [[#449](https://github.com/extphprs/ext-php-rs/issues/449)] 
- Enforce doc comments for `ext-php-rs` (by @Xenira) [[#392](https://github.com/extphprs/ext-php-rs/issues/392)] 

## [0.13.1](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.13.0...ext-php-rs-v0.13.1) - 2025-02-13

### Fixed
- *(array)* Fix double ended iterator implementation (#351) (by @Xenira) [[#351](https://github.com/extphprs/ext-php-rs/issues/351)] [[#316](https://github.com/extphprs/ext-php-rs/issues/316)] 
- *(globals)* Correctly fetch `$_REQUEST` super global (#334) (by @Xenira) [[#334](https://github.com/extphprs/ext-php-rs/issues/334)] [[#331](https://github.com/extphprs/ext-php-rs/issues/331)] 

### Other
- *(cfg)* Change php81 cfg to 8.1+ (#365) (by @Xenira) [[#365](https://github.com/extphprs/ext-php-rs/issues/365)] 

## [0.13.0](https://github.com/extphprs/ext-php-rs/compare/ext-php-rs-v0.12.0...ext-php-rs-v0.13.0) - 2025-02-06

### Fixed
- *(array)* Fix null dereference in iterator (#358) (by @Xenira) [[#358](https://github.com/extphprs/ext-php-rs/issues/358)] [[#357](https://github.com/extphprs/ext-php-rs/issues/357)] 
- *(globals)* [**breaking**] Disabled `$_REQUEST` super global function (#332) (by @Xenira) [[#332](https://github.com/extphprs/ext-php-rs/issues/332)] [[#331](https://github.com/extphprs/ext-php-rs/issues/331)] 
> If you used `http_request_vars()` before it will now panic until a proper implementation is found.

### Other
- *(clippy)* Fix new clippy checks (#352) (by @Xenira) [[#352](https://github.com/extphprs/ext-php-rs/issues/352)] 
- *(clippy)* Fix new clippy errors (by @Xenira)
- *(php)* Add deprecation warning for php 8.0 (#353) (by @Xenira) [[#353](https://github.com/extphprs/ext-php-rs/issues/353)] [[#343](https://github.com/extphprs/ext-php-rs/issues/343)] 
- *(release)* Add release bot (#346) (by @Xenira) [[#346](https://github.com/extphprs/ext-php-rs/issues/346)] [[#340](https://github.com/extphprs/ext-php-rs/issues/340)] 
- *(windows)* Add a debug build with debugging symbols (#350) (by @EdmondDantes) [[#350](https://github.com/extphprs/ext-php-rs/issues/350)] 
- Fix typos (by @Xenira)
- Windows build should try archives (by @joehoyle)
- Fmt (by @joehoyle)
- Use vs17 on php 8.4+ (by @joehoyle)
- Don't use archive for 8.4.1 (by @joehoyle)
- Fmt (by @joehoyle)
- Support php 8.4 internal api changes (by @joehoyle)
- PHP 8.4 (by @joehoyle)
- Fix pipeline (#320) (by @Xenira) [[#320](https://github.com/extphprs/ext-php-rs/issues/320)] 
- Update README.md ([#317](https://github.com/extphprs/ext-php-rs/pull/317)) (by @s00d) [[#317](https://github.com/extphprs/ext-php-rs/issues/317)] 

## 0.10.1
- chore: Bitflags upgrade to v2 by @ptondereau [#221]
- chore: Update to bindgen 0.65.1 @ptondereau [#220]
- fix: Switch to use zend apis for array iteration by @joehoyle [#219]
- docs: Fix some typos + badges by @striezel [#218]
- fix: Stop watching Cargo.lock for changes by @rmccue [#217]
- fix: Fix Zval IS_PTR type detection by @joehoyle [#216]
- feat: Pass args to startup function by @joehoyle [#215]
- chore: Mate GlobalExecutor::get_mut() public by @joehoyle [#214]
- feat: Add is_identical for zvals by @Christian-Rades [#213]

[#213]: https://github.com/extphprs/ext-php-rs/pull/217
[#214]: https://github.com/extphprs/ext-php-rs/pull/227
[#215]: https://github.com/extphprs/ext-php-rs/pull/226
[#216]: https://github.com/extphprs/ext-php-rs/pull/223
[#217]: https://github.com/extphprs/ext-php-rs/pull/232
[#218]: https://github.com/extphprs/ext-php-rs/pull/234
[#219]: https://github.com/extphprs/ext-php-rs/pull/240
[#220]: https://github.com/extphprs/ext-php-rs/pull/241
[#221]: https://github.com/extphprs/ext-php-rs/pull/242

## 0.10.0
- feat: Add PHP 8.2 support by @ptondereau [#212]

[#212]: https://github.com/extphprs/ext-php-rs/pull/212

## Version 0.9.0

- ci+docs: honour PHP_CONFIG & rebuild automatically when env vars change by @julius [#210]
- chore: Update generated FFI bindings with bindgen 0.63 by @ptondereau [#211]

**BC changes**
- feat: allows ZendStr to contain null bytes by @julius [#202]

**Migration**
See: [#202]

[#202]: https://github.com/extphprs/ext-php-rs/pull/202
[#210]: https://github.com/extphprs/ext-php-rs/pull/210
[#211]: https://github.com/extphprs/ext-php-rs/pull/211


## Version 0.8.3

- build: Check docs warnings in CI by @davidcole1340 in [#180]
- fix: Fixes infinite loop in ClassEntry::instance_of() by @ju1ius in [#188]
- fix: Fix binary slice lifetimes by @davidcole1340 in [#181]
- build: Fixes CI workflow configuration by @ju1ius in [#195]
- feat: Add get_id() and hash() methods on ZendObject by @ju1ius in [#196]
- docs: Describes restrictions on generic parameters for `php_class` by @ju1ius in [#194]
- feat: Add instance_of() and get_class_entry() methods on ZendObject by @ju1ius in [#197]

[#180]: https://github.com/extphprs/ext-php-rs/pull/180
[#188]: https://github.com/extphprs/ext-php-rs/pull/188
[#181]: https://github.com/extphprs/ext-php-rs/pull/181
[#195]: https://github.com/extphprs/ext-php-rs/pull/195
[#196]: https://github.com/extphprs/ext-php-rs/pull/196
[#194]: https://github.com/extphprs/ext-php-rs/pull/194
[#197]: https://github.com/extphprs/ext-php-rs/pull/197

## Version 0.8.2

- Update changelog for latest versions by @striezel in [#161]
- fix building docs on docs.rs by @davidcole1340 in [#165]
- Add some standard zend interfaces by @nikeee in [#164]
- Correct parameter name. by @denzyldick in [#168]
- fix describe when using `#[implements]` by @davidcole1340 in [#169]
- Add example that shows how to implement an interface by @nikeee in [#167]
- add `before` flag to `#[php_startup]` by @davidcole1340 in [#170]
- add ability to define abstract methods by @davidcole1340 in [#171]
- chore(cli): Bump Clap for CLI tool by @ptondereau in [#177]
- fix type links in docs.rs by @davidcole1340 in [#179]

[#161]: https://github.com/extphprs/ext-php-rs/pull/161
[#165]: https://github.com/extphprs/ext-php-rs/pull/165
[#164]: https://github.com/extphprs/ext-php-rs/pull/164
[#168]: https://github.com/extphprs/ext-php-rs/pull/168
[#169]: https://github.com/extphprs/ext-php-rs/pull/169
[#167]: https://github.com/extphprs/ext-php-rs/pull/167
[#170]: https://github.com/extphprs/ext-php-rs/pull/170
[#171]: https://github.com/extphprs/ext-php-rs/pull/171
[#177]: https://github.com/extphprs/ext-php-rs/pull/177
[#179]: https://github.com/extphprs/ext-php-rs/pull/179

## Version 0.8.1

- 404 /guide doesn't exists. by @denzyldick in [#149]
- Fixed some typos by @denzyldick in [#148]
- Fix a few typos by @striezel in [#150]
- fix causes of some clippy warnings by @striezel in [#152]
- fix more causes of clippy warnings by @striezel in [#157]
- attempt to fix errors related to clap by @striezel in [#158]
- ci: run clippy only on stable Rust channel by @striezel in [#159]
- update actions/checkout in GitHub Actions workflows to v3 by @striezel in
  [#151]
- Add ability to set function name on php_function macro by @joehoyle in [#153]
- Specify classes as fully-qualified names in stubs by @joehoyle in [#156]
- Support marking classes as interfaces by @joehoyle in [#155]
- Support marking methods as abstract by @joehoyle in [#154]
- Add php-scrypt as a example project by @PineappleIOnic in [#146]
- Fix ini file duplication and truncation when using cargo-php command by
  @roborourke in [#136]
- Allow passing --yes parameter to bypass prompts by @roborourke in [#135]

[#135]: https://github.com/extphprs/ext-php-rs/pull/135
[#136]: https://github.com/extphprs/ext-php-rs/pull/136
[#146]: https://github.com/extphprs/ext-php-rs/pull/146
[#148]: https://github.com/extphprs/ext-php-rs/pull/148
[#149]: https://github.com/extphprs/ext-php-rs/pull/149
[#150]: https://github.com/extphprs/ext-php-rs/pull/150
[#151]: https://github.com/extphprs/ext-php-rs/pull/151
[#152]: https://github.com/extphprs/ext-php-rs/pull/152
[#153]: https://github.com/extphprs/ext-php-rs/pull/153
[#154]: https://github.com/extphprs/ext-php-rs/pull/154
[#155]: https://github.com/extphprs/ext-php-rs/pull/155
[#156]: https://github.com/extphprs/ext-php-rs/pull/156
[#157]: https://github.com/extphprs/ext-php-rs/pull/157
[#158]: https://github.com/extphprs/ext-php-rs/pull/158
[#159]: https://github.com/extphprs/ext-php-rs/pull/159

## Version 0.8.0

- Windows support by @davidcole1340 in [#128]
- Support for binary slice to avoid extra allocation by @TobiasBengtsson in
  [#139]
- Bump dependencies by @ptondereau in [#144]

[#128]: https://github.com/extphprs/ext-php-rs/pull/128
[#139]: https://github.com/extphprs/ext-php-rs/pull/139
[#144]: https://github.com/extphprs/ext-php-rs/pull/144

## Version 0.7.4

- Fix is_true() / is_false() in Zval by @joehoyle in [#116]
- readme: fix link to guide by @TorstenDittmann in [#120]
- Fix request_(startup|shutdown)_function in ModuleBuilder by @glyphpoch in
  [#119]
- Fix CI on macOS by @davidcole1340 in [#126]
- Add ability to pass modifier function for classes by @davidcole1340 in [#127]

[#116]: https://github.com/extphprs/ext-php-rs/pull/116
[#119]: https://github.com/extphprs/ext-php-rs/pull/119
[#120]: https://github.com/extphprs/ext-php-rs/pull/120
[#126]: https://github.com/extphprs/ext-php-rs/pull/126
[#127]: https://github.com/extphprs/ext-php-rs/pull/127

## Version 0.7.3

- Upgrade `clap` to `3.0.0-rc3`. [#113]
- Build properties hashmap once and cache inside class metadata. [#114]
- Add `impl FromZval for &Zval` and `impl FromZvalMut for &mut Zval`.
- Add `has_numerical_keys` and `has_sequential_keys` to `ZendHashTable`. [#115]

Thanks to the following contributors:

- @davidcole1340
- @vkill

[#113]: https://github.com/extphprs/ext-php-rs/pull/113
[#114]: https://github.com/extphprs/ext-php-rs/pull/114
[#115]: https://github.com/extphprs/ext-php-rs/pull/115

## Version 0.7.2

- Add preliminary PHP 8.1 support. [#109]
  - Extensions should now compile for PHP 8.1. This doesn't implement any of the
    new PHP 8.1 features.
- Add `anyhow` cargo feature to implement
  `From<anyhow::Error> for PhpException`. [#110]
- Made `ClassMetadata: Send + Sync`. [#111]
- Fixed registering constants with expressions. [#112]

[#109]: https://github.com/extphprs/ext-php-rs/pull/109
[#110]: https://github.com/extphprs/ext-php-rs/pull/110
[#111]: https://github.com/extphprs/ext-php-rs/pull/111
[#112]: https://github.com/extphprs/ext-php-rs/pull/112

## Version 0.7.1

- Ensure stable ABI between `cargo-php` and downstream extensions. [#108]
  - `ext-php-rs` versions used when compiling CLI and extension are now
    compared.

[#108]: https://github.com/extphprs/ext-php-rs/pull/108

## Version 0.7.0

- Disabled serialization and unserialization of Rust structs exported as PHP
  classes. [#105]
  - You can't serialize an associated Rust struct so this would have never
    worked, but disabling them fixes crashes when running in an environment like
    psysh.
- Replaced boxed module inside `ModuleBuilder` with in-struct module.
- Fixed builds failing on Linux AArch64 systems. [#106]
- Added `cargo-php` for creating stubs, installing and uninstalling extensions.
  [#107]
  - Check out the guide for more information on this.

[#105]: https://github.com/extphprs/ext-php-rs/pull/105
[#106]: https://github.com/extphprs/ext-php-rs/pull/106
[#107]: https://github.com/extphprs/ext-php-rs/pull/107

## Version 0.6.0

- Reorganized project. [#101]
  - Changed (almost all) module paths. Too many changes to list them all, check
    out the docs.
  - Removed `skel` project.
- Allow methods to accept references to `ZendClassObject<T>` instead of `self`.
  [#103]

[#101]: https://github.com/extphprs/ext-php-rs/pull/101
[#103]: https://github.com/extphprs/ext-php-rs/pull/103

## Version 0.5.3

- Fixed docs.rs PHP bindings file.

## Version 0.5.2

- Constructors that return `Self` can now be added to classes. [#83]
  - `Default` is no longer required to be implemented on classes, however, a
    constructor must be specified if you want to construct the class from PHP.
  - Constructors can return `Self` or `Result<Self, E>`, where
    `E: Into<PhpException>`.
- Added `FromZendObject` and `IntoZendObject` traits. [#74]
- Added `#[derive(ZvalConvert)]` derive macro. Derives `IntoZval` and `FromZval`
  on arbitrary structs and enums. [#78]
- Added `ZBox<T>`, similar to `Box<T>`, to allocate on the Zend heap. [#94]
- Changed execution data functions to take mutable references. [#100]
- `&mut T` is now valid as a function parameter. [#100]

Thanks to the contributors for this release:

- @davidcole1340
- @vodik

[#74]: https://github.com/extphprs/ext-php-rs/pull/74
[#78]: https://github.com/extphprs/ext-php-rs/pull/78
[#83]: https://github.com/extphprs/ext-php-rs/pull/83
[#94]: https://github.com/extphprs/ext-php-rs/pull/94
[#100]: https://github.com/extphprs/ext-php-rs/pull/100

## Version 0.5.1

- `PhpException` no longer requires a lifetime [#80].
- Added `PhpException` and `PhpResult` to prelude [#80].
- Fixed `ZendString` missing last character [#82].

[#80]: https://github.com/extphprs/ext-php-rs/pull/80
[#82]: https://github.com/extphprs/ext-php-rs/pull/82

## Version 0.5.0

### Breaking changes

- Method names are now renamed to snake case by default [#63].
- Refactored `ZendHashTable` into an owned and borrowed variant [#76].
  - Creating a new hashtable is done through the `OwnedHashTable` struct, which
    is then dereferenced to `&HashTable`, as `String` is to `&str`.
- Refactored `ZendString` into an owned and borrowed variant [#77].
  - Creating a new Zend string is done through the `ZendString` struct, which is
    then dereferenced to `&ZendStr`, as `String` is to `&str`.
- Class properties are now defined as struct properties, removing the old
  property system in the process [#69].

### Enhancements

- Added interfaces and parent class to the `Debug` implementation for
  `ClassEntry` [@72b0491].
- Rust unit type `()` now has a datatype of `void` [@8b3ed08].
- Functions returning Rust objects will now display their full classname in
  reflection [#64].
- Fixed alignment of class objects in memory [#66].

Thanks to the contributors for this release:

- @davidcole1340
- @vodik

[#63]: https://github.com/extphprs/ext-php-rs/pull/63
[#76]: https://github.com/extphprs/ext-php-rs/pull/76
[#77]: https://github.com/extphprs/ext-php-rs/pull/77
[#69]: https://github.com/extphprs/ext-php-rs/pull/69
[#64]: https://github.com/extphprs/ext-php-rs/pull/64
[#66]: https://github.com/extphprs/ext-php-rs/pull/66
[@72b0491]: https://github.com/extphprs/ext-php-rs/commit/72b0491
[@8b3ed08]: https://github.com/extphprs/ext-php-rs/commit/8b3ed08

## Version 0.2.0 - 0.4.0

- Added macros!
- Missed a bit :(

## Version 0.1.0

- `Zval::reference()` returns a reference instead of a dereferenced pointer.
- Added `ZendHashTable::iter()` - note this is changing in a future version.
- `ClassBuilder::extends()` now takes a reference rather than a pointer to match
  the return type of `ClassEntry::exception()`.
- `ClassEntry::build()` now returns a reference - same reason as above.
- Improve library 'safety' by removing `unwrap` calls:
  - `.build()` returns `Result` on `FunctionBuilder`, `ClassBuilder` and
    `ModuleBuilder`.
  - `.property()` and `.constant()` return `Result` on `ClassBuilder`.
  - `.register_constant()` returns `Result`.
  - `.try_call()` on callables now return `Result` rather than `Option`.
  - `throw()` and `throw_with_code()` now returns `Result`.
  - `new()` and `new_interned()` on `ZendString` now returns a `Result`.
  - For `ZendHashTable`:
    - `insert()`, `insert_at_index()` now returns a
      `Result<HashTableInsertResult>`, where `Err` failed, `Ok(Ok)` inserts
      successfully without overwrite, and `Ok(OkWithOverwrite(&Zval))` inserts
      successfully with overwrite.
    - `push()` now returns a `Result`.
    - Converting from a `Vec` or `HashMap` to a `ZendHashTable` is fallible, so
      it now implements `TryFrom` as opposed to `From`.
  - For `Zval`:
    - `set_string()` now returns a `Result`, and takes a second parameter
      (persistent).
    - `set_persistent_string()` has now been removed in favour of
      `set_string()`.
    - `set_interned_string()` also returns a `Result`.
    - `set_array()` now only takes a `ZendHashTable`, you must convert your
      `Vec` or `HashMap` by calling `try_into()` and handling the error.

## Version 0.0.7

- Added support for thread-safe PHP (@davidcole1340) #37
- Added ability to add properties to classes (@davidcole1340) #39
- Added better interactions with objects (@davidcole1340) #41

## Version 0.0.6

- Fixed `panic!` when a PHP binary string was given to a function
  (@davidcole1340) [c:d73788e]
- Fixed memory leak when returning an array from Rust to PHP (@davidcole1340)
  #34
- Documentation is now deployed to
  [GitHub Pages](https://ext-php.rs) (@davidcole1340) #35
- Added ability to unpack and pack binary strings similar to PHP
  (@davidcole1340) #32
- Allowed `default-features` to be true for Bindgen (@willbrowningme) #36

## Version 0.0.5

- Relicensed project under MIT or Apache 2.0 as per Rust crate guidelines
  (@davidcole1340) [c:439f2ae]
- Added `parse_args!` macro to simplify argument parsing (@davidcole1340)
  [c:45c7242]
- Added ability to throw exceptions from Rust to PHP (@davidcole1340)
  [c:45c7242]
- Added ability to register global constants (@davidcole1340) [c:472e26e]
- Implemented `From<ZendHashTable>` for `Vec` (@davidcole1340) [c:3917c41]
- Expanded implementations for converting to `Zval` from primitives
  (@davidcole1340) [c:d4c6aa2]
- Replaced unit errors with an `Error` enum (@davidcole1340) [c:f11451f]
- Added `Debug` and `Clone` implementations for most structs (@davidcole1340)
  [c:62a43e6]
