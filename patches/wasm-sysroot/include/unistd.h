// Minimal unistd.h for wasm32-unknown-unknown
#ifndef _UNISTD_H
#define _UNISTD_H
#include <stddef.h>
// WASM has no POSIX - stub out what tree-sitter needs
#define STDOUT_FILENO 1
#define STDERR_FILENO 2
int dup(int);
int close(int);
#endif
