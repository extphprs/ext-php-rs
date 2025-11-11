#include "zend.h"
#include "sapi/embed/php_embed.h"
#ifdef EXT_PHP_RS_PHP_82
#include "php_ini_builder.h"
#endif

void* ext_php_rs_embed_callback(int argc, char** argv, void* (*callback)(void *), void *ctx);

void ext_php_rs_sapi_startup();
void ext_php_rs_sapi_shutdown();
void ext_php_rs_sapi_per_thread_init();
void ext_php_rs_sapi_per_thread_shutdown();

void ext_php_rs_php_error(int type, const char *format, ...);
