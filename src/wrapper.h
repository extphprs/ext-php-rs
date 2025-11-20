// PHP for Windows uses the `vectorcall` calling convention on some functions.
// This is guarded by the `ZEND_FASTCALL` macro, which is set to `__vectorcall`
// on Windows and nothing on other systems.
//
// However, `ZEND_FASTCALL` is only set when compiling with MSVC and the PHP
// source code checks for the __clang__ macro and will not define `__vectorcall`
// if it is set (even on Windows). This is a problem as Bindgen uses libclang to
// generate bindings. To work around this, we include the header file containing
// the `ZEND_FASTCALL` macro but not before undefining `__clang__` to pretend we
// are compiling on MSVC.
#if defined(_MSC_VER) && defined(__clang__)
// PHP 8.5+ uses safe integer arithmetic functions. For Clang, we define macros
// to use Clang's built-in overflow detection instead of Windows' intsafe.h.
// This matches PHP's upstream solution in https://github.com/php/php-src/pull/17472
#if defined(EXT_PHP_RS_PHP_85)
#define PHP_HAVE_BUILTIN_SADDL_OVERFLOW 1
#define PHP_HAVE_BUILTIN_SADDLL_OVERFLOW 1
#define PHP_HAVE_BUILTIN_SSUBL_OVERFLOW 1
#define PHP_HAVE_BUILTIN_SSUBLL_OVERFLOW 1
#define PHP_HAVE_BUILTIN_SMULL_OVERFLOW 1
#define PHP_HAVE_BUILTIN_SMULLL_OVERFLOW 1
#endif
#undef __clang__
#include "zend_portability.h"
#define __clang__
#endif

#include "php.h"

#include "ext/standard/info.h"
#include "ext/standard/php_var.h"
#include "ext/standard/file.h"
#ifdef EXT_PHP_RS_PHP_81
#include "zend_enum.h"
#endif
#include "zend_exceptions.h"
#include "zend_inheritance.h"
#include "zend_interfaces.h"
#include "php_variables.h"
#include "zend_ini.h"
#include "main/SAPI.h"

zend_string *ext_php_rs_zend_string_init(const char *str, size_t len, bool persistent);
void ext_php_rs_zend_string_release(zend_string *zs);
bool ext_php_rs_is_known_valid_utf8(const zend_string *zs);
void ext_php_rs_set_known_valid_utf8(zend_string *zs);

const char *ext_php_rs_php_build_id();
void *ext_php_rs_zend_object_alloc(size_t obj_size, zend_class_entry *ce);
void ext_php_rs_zend_object_release(zend_object *obj);
zend_executor_globals *ext_php_rs_executor_globals();
php_core_globals *ext_php_rs_process_globals();
sapi_globals_struct *ext_php_rs_sapi_globals();
php_file_globals *ext_php_rs_file_globals();
sapi_module_struct *ext_php_rs_sapi_module();
bool ext_php_rs_zend_try_catch(void* (*callback)(void *), void *ctx, void **result);
bool ext_php_rs_zend_first_try_catch(void* (*callback)(void *), void *ctx, void **result);
void ext_php_rs_zend_bailout();
