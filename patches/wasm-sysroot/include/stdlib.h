// stdlib.h for wasm32-unknown-unknown
#ifndef _STDLIB_H
#define _STDLIB_H

#include <stddef.h>

#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1

#define RAND_MAX 2147483647

void *malloc(size_t);
void *calloc(size_t, size_t);
void *realloc(void *, size_t);
void free(void *);

_Noreturn void abort(void);
_Noreturn void exit(int);
_Noreturn void _Exit(int);
int atexit(void (*)(void));

int atoi(const char *);
long atol(const char *);
double atof(const char *);
long strtol(const char *, char **, int);
unsigned long strtoul(const char *, char **, int);
long long strtoll(const char *, char **, int);
unsigned long long strtoull(const char *, char **, int);
double strtod(const char *, char **);
float strtof(const char *, char **);

int abs(int);
long labs(long);
long long llabs(long long);

typedef struct { int quot; int rem; } div_t;
typedef struct { long quot; long rem; } ldiv_t;
typedef struct { long long quot; long long rem; } lldiv_t;
div_t div(int, int);
ldiv_t ldiv(long, long);
lldiv_t lldiv(long long, long long);

void qsort(void *, size_t, size_t, int (*)(const void *, const void *));
void *bsearch(const void *, const void *, size_t, size_t, int (*)(const void *, const void *));

int rand(void);
void srand(unsigned int);

char *getenv(const char *);

int mblen(const char *, size_t);
size_t mbstowcs(wchar_t *, const char *, size_t);
size_t wcstombs(char *, const wchar_t *, size_t);

#endif
