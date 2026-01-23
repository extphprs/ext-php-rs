#include "wrapper.h"

zend_string *ext_php_rs_zend_string_init(const char *str, size_t len, bool persistent) {
  return zend_string_init(str, len, persistent);
}

void ext_php_rs_zend_string_release(zend_string *zs) {
  zend_string_release(zs);
}

bool ext_php_rs_is_known_valid_utf8(const zend_string *zs) {
  return GC_FLAGS(zs) & IS_STR_VALID_UTF8;
}

void ext_php_rs_set_known_valid_utf8(zend_string *zs) {
  if (!ZSTR_IS_INTERNED(zs)) {
    GC_ADD_FLAGS(zs, IS_STR_VALID_UTF8);
  }
}

const char *ext_php_rs_php_build_id() { return ZEND_MODULE_BUILD_ID; }

void *ext_php_rs_zend_object_alloc(size_t obj_size, zend_class_entry *ce) {
  return zend_object_alloc(obj_size, ce);
}

void ext_php_rs_zend_object_release(zend_object *obj) {
  zend_object_release(obj);
}

zend_executor_globals *ext_php_rs_executor_globals() {
#ifdef ZTS
#ifdef ZEND_ENABLE_STATIC_TSRMLS_CACHE
  return TSRMG_FAST_BULK_STATIC(executor_globals_offset, zend_executor_globals);
#else
  return TSRMG_FAST_BULK(executor_globals_offset, zend_executor_globals *);
#endif
#else
  return &executor_globals;
#endif
}

zend_compiler_globals *ext_php_rs_compiler_globals() {
#ifdef ZTS
#ifdef ZEND_ENABLE_STATIC_TSRMLS_CACHE
  return TSRMG_FAST_BULK_STATIC(compiler_globals_offset, zend_compiler_globals);
#else
  return TSRMG_FAST_BULK(compiler_globals_offset, zend_compiler_globals *);
#endif
#else
  return &compiler_globals;
#endif
}

php_core_globals *ext_php_rs_process_globals() {
#ifdef ZTS
#ifdef ZEND_ENABLE_STATIC_TSRMLS_CACHE
  return TSRMG_FAST_BULK_STATIC(core_globals_offset, php_core_globals);
#else
  return TSRMG_FAST_BULK(core_globals_offset, php_core_globals *);
#endif
#else
  return &core_globals;
#endif
}

sapi_globals_struct *ext_php_rs_sapi_globals() {
#ifdef ZTS
#ifdef ZEND_ENABLE_STATIC_TSRMLS_CACHE
  return TSRMG_FAST_BULK_STATIC(sapi_globals_offset, sapi_globals_struct);
#else
  return TSRMG_FAST_BULK(sapi_globals_offset, sapi_globals_struct *);
#endif
#else
  return &sapi_globals;
#endif
}

php_file_globals *ext_php_rs_file_globals() {
#ifdef ZTS
  return TSRMG_FAST_BULK(file_globals_id, php_file_globals *);
#else
  return &file_globals;
#endif
}

sapi_module_struct *ext_php_rs_sapi_module() {
  return &sapi_module;
}

bool ext_php_rs_zend_try_catch(void* (*callback)(void *), void *ctx, void **result) {
  zend_try {
    *result = callback(ctx);
  } zend_catch {
    return true;
  } zend_end_try();

  return false;
}

bool ext_php_rs_zend_first_try_catch(void* (*callback)(void *), void *ctx, void **result) {
  zend_first_try {
    *result = callback(ctx);
  } zend_catch {
    return true;
  } zend_end_try();

  return false;
}

void ext_php_rs_zend_bailout() {
  zend_bailout();
}

zend_op_array *ext_php_rs_zend_compile_string(zend_string *source, const char *filename) {
#if PHP_VERSION_ID >= 80200
  return zend_compile_string(source, filename, ZEND_COMPILE_POSITION_AFTER_OPEN_TAG);
#else
  return zend_compile_string(source, filename);
#endif
}

void ext_php_rs_zend_execute(zend_op_array *op_array) {
  zval local_retval;
  ZVAL_UNDEF(&local_retval);

  op_array->scope = zend_get_executed_scope();

  zend_try {
    zend_execute(op_array, &local_retval);
  } zend_catch {
    destroy_op_array(op_array);
    efree_size(op_array, sizeof(zend_op_array));
    zend_bailout();
  } zend_end_try();

  zval_ptr_dtor(&local_retval);
  zend_destroy_static_vars(op_array);
  destroy_op_array(op_array);
  efree_size(op_array, sizeof(zend_op_array));
}

void *ext_php_rs_pemalloc(size_t size) {
  return pemalloc(size, 1);
}
