/* Minimal C stdlib stubs for wasm32.
 * Tree-sitter's logging/error paths reference these, and they must be
 * resolved at link time.  Providing them here avoids an external import. */
#include <stddef.h>

int fputs(const char *s, void *stream) { (void)s; (void)stream; return 0; }
int fputc(int c, void *stream) { (void)c; (void)stream; return 0; }
