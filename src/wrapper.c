#include "wrapper.h"

#pragma weak zend_empty_string
#pragma weak zend_one_char_string

zend_string *ext_php_rs_zend_string_init(const char *str, size_t len, bool persistent) {
  if (!persistent && len <= 1
      && zend_one_char_string != NULL && zend_one_char_string[0] != NULL) {
    return len == 0 ? zend_empty_string : ZSTR_CHAR((zend_uchar) *str);
  }
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

#ifdef ZTS
void *ext_php_rs_tsrmg_bulk(int id) {
  return TSRMG_BULK(id, void *);
}
#endif

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
    efree(op_array);
    zend_bailout();
  } zend_end_try();

  zval_ptr_dtor(&local_retval);
  zend_destroy_static_vars(op_array);
  destroy_op_array(op_array);
  efree(op_array);
}

#if PHP_VERSION_ID >= 80300
void _ext_php_rs_zend_fcc_addref(zend_fcall_info_cache *fcc) {
  zend_fcc_addref(fcc);
}

void _ext_php_rs_zend_fcc_dtor(zend_fcall_info_cache *fcc) {
  zend_fcc_dtor(fcc);
}
#else
void _ext_php_rs_zend_fcc_addref(zend_fcall_info_cache *fcc) {
  if (fcc->function_handler &&
      (fcc->function_handler->common.fn_flags & ZEND_ACC_CALL_VIA_TRAMPOLINE) &&
      fcc->function_handler == &EG(trampoline)) {
    zend_function *copy = emalloc(sizeof(zend_function));
    memcpy(copy, fcc->function_handler, sizeof(zend_function));
    fcc->function_handler->common.function_name = NULL;
    fcc->function_handler = copy;
  }
  if (fcc->object) {
    GC_ADDREF(fcc->object);
  }
}

void _ext_php_rs_zend_fcc_dtor(zend_fcall_info_cache *fcc) {
  if (fcc->object) {
    OBJ_RELEASE(fcc->object);
  }
  zend_release_fcall_info_cache(fcc);
  memset(fcc, 0, sizeof(*fcc));
}
#endif

int _ext_php_rs_cached_call_function(zend_fcall_info_cache *fcc, zval *retval, uint32_t param_count, zval *params, HashTable *named_params) {
  zend_fcall_info fci;

  ZVAL_UNDEF(&fci.function_name);
  fci.size = sizeof(fci);
  fci.object = fcc->object;
  fci.retval = retval;
  fci.param_count = param_count;
  fci.params = params;
  fci.named_params = named_params;

  return zend_call_function(&fci, fcc);
}
