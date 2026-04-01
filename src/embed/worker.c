#include "embed.h"
#include "php_output.h"
#include "SAPI.h"
#include "php_main.h"
#include "zend_compile.h"

void ext_php_rs_worker_request_shutdown(void) {
  php_output_end_all();
  php_output_deactivate();
  sapi_deactivate_module();
  sapi_deactivate_destroy();
}

int ext_php_rs_worker_request_startup(void) {
  php_output_activate();
  sapi_activate();
  zend_activate_auto_globals();
  return SUCCESS;
}

void ext_php_rs_worker_reset_superglobals(void) {
  zend_is_auto_global(zend_string_init_interned("_SERVER", sizeof("_SERVER") - 1, 0));
  zend_is_auto_global(zend_string_init_interned("_GET", sizeof("_GET") - 1, 0));
  zend_is_auto_global(zend_string_init_interned("_POST", sizeof("_POST") - 1, 0));
  zend_is_auto_global(zend_string_init_interned("_COOKIE", sizeof("_COOKIE") - 1, 0));
  zend_is_auto_global(zend_string_init_interned("_REQUEST", sizeof("_REQUEST") - 1, 0));
  zend_is_auto_global(zend_string_init_interned("_ENV", sizeof("_ENV") - 1, 0));
  zend_is_auto_global(zend_string_init_interned("_FILES", sizeof("_FILES") - 1, 0));
}
