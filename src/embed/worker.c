#include "embed.h"
#include "php_output.h"
#include "SAPI.h"
#include "php_main.h"
#include "zend_compile.h"
#include "zend_exceptions.h"

static bool ext_php_rs_worker_needs_restart(void) {
#ifdef ZEND_HAS_OOM
  if (ZEND_HAS_OOM()) {
    return true;
  }
#endif
  return CG(unclean_shutdown);
}

void ext_php_rs_worker_request_shutdown(void) {
  zend_clear_exception();
  if (ext_php_rs_worker_needs_restart()) {
    php_request_shutdown(NULL);
    return;
  }
  php_output_end_all();
  php_output_deactivate();
  sapi_deactivate_module();
  sapi_deactivate_destroy();
}

int ext_php_rs_worker_request_startup(void) {
  if (EG(flags) & EG_FLAGS_IN_SHUTDOWN) {
    return php_request_startup();
  }
  php_output_activate();
  sapi_activate();
  zend_activate_auto_globals();
  return SUCCESS;
}

void ext_php_rs_worker_reset_superglobals(void) {
  zend_is_auto_global_str(ZEND_STRL("_SERVER"));
  zend_is_auto_global_str(ZEND_STRL("_GET"));
  zend_is_auto_global_str(ZEND_STRL("_POST"));
  zend_is_auto_global_str(ZEND_STRL("_COOKIE"));
  zend_is_auto_global_str(ZEND_STRL("_REQUEST"));
  zend_is_auto_global_str(ZEND_STRL("_ENV"));
  zend_is_auto_global_str(ZEND_STRL("_FILES"));
}
