// Minimal stdio.h for wasm32-unknown-unknown
#ifndef _STDIO_H
#define _STDIO_H

#include <stddef.h>
#include <stdarg.h>

typedef struct FILE FILE;
extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

int fprintf(FILE *, const char *, ...);
int vfprintf(FILE *, const char *, va_list);
int vsnprintf(char *, size_t, const char *, va_list);
int snprintf(char *, size_t, const char *, ...);
int sprintf(char *, const char *, ...);
int fputc(int, FILE *);
int fputs(const char *, FILE *);
int fclose(FILE *);
FILE *fdopen(int, const char *);

#endif
