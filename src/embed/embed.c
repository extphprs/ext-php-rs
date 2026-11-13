#include "embed.h"

// We actually use the PHP embed API to run PHP code in test
// At some point we might want to use our own SAPI to do that
void* ext_php_rs_embed_callback(int argc, char** argv, void* (*callback)(void *), void *ctx) {
  void *result = NULL;

  PHP_EMBED_START_BLOCK(argc, argv)

  result = callback(ctx);

  PHP_EMBED_END_BLOCK()

  return result;
}

zend_result ext_php_rs_eval_string(const char *str, zval *retval, const char *name) {
  zend_string *source = zend_string_concat3("return ", sizeof("return ") - 1, str, strlen(str), ";", sizeof(";") - 1);
  uint32_t compiler_options = CG(compiler_options);
  CG(compiler_options) = ZEND_COMPILE_DEFAULT_FOR_EVAL;
#if PHP_VERSION_ID >= 80200
  zend_op_array *op_array = zend_compile_string(source, name, ZEND_COMPILE_POSITION_AFTER_OPEN_TAG);
#else
  zend_op_array *op_array = zend_compile_string(source, name);
#endif
  CG(compiler_options) = compiler_options;
  zend_string_release(source);

  if (!op_array) {
    return FAILURE;
  }

  zval local_retval;
  ZVAL_UNDEF(&local_retval);
  EG(no_extensions) = 1;
  op_array->scope = zend_get_executed_scope();
  zend_execute(op_array, &local_retval);
  EG(no_extensions) = 0;

  if (Z_TYPE(local_retval) == IS_UNDEF) {
    ZVAL_NULL(retval);
  } else {
    ZVAL_COPY_VALUE(retval, &local_retval);
  }

  zend_destroy_static_vars(op_array);
  destroy_op_array(op_array);
  efree_size(op_array, sizeof(zend_op_array));
  return SUCCESS;
}

void ext_php_rs_sapi_startup() {
  #if defined(SIGPIPE) && defined(SIG_IGN)
    signal(SIGPIPE, SIG_IGN);
  #endif

  #ifdef ZTS
    php_tsrm_startup();
    #ifdef PHP_WIN32
      ZEND_TSRMLS_CACHE_UPDATE();
    #endif
  #endif

  zend_signal_startup();
}

SAPI_API void ext_php_rs_sapi_shutdown() {
  #ifdef ZTS
    tsrm_shutdown();
  #endif
}

SAPI_API void ext_php_rs_sapi_per_thread_init() {
  #ifdef ZTS
    (void)ts_resource(0);
    #ifdef PHP_WIN32
      ZEND_TSRMLS_CACHE_UPDATE();
    #endif
  #endif
}

SAPI_API void ext_php_rs_sapi_per_thread_shutdown() {
  #ifdef ZTS
    ts_free_thread();
  #endif
}

// Wrap `php_ini_builder_deinit` as it's `static inline` which gets discarded
// by cbindgen.
#ifdef EXT_PHP_RS_PHP_82
void ext_php_rs_php_ini_builder_deinit(struct php_ini_builder *b) {
  php_ini_builder_deinit(b);
}
#endif
