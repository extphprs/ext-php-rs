#include "zend.h"
#include "sapi/embed/php_embed.h"
#ifdef EXT_PHP_RS_PHP_82
#include "php_ini_builder.h"
#endif

void* ext_php_rs_embed_callback(int argc, char** argv, void* (*callback)(void *), void *ctx);

zend_result ext_php_rs_eval_string(const char *str, zval *retval, const char *name);

void ext_php_rs_sapi_startup();
void ext_php_rs_sapi_shutdown();
void ext_php_rs_sapi_per_thread_init();
void ext_php_rs_sapi_per_thread_shutdown();

void ext_php_rs_worker_request_shutdown(void);
int ext_php_rs_worker_request_startup(void);
void ext_php_rs_worker_reset_superglobals(void);
