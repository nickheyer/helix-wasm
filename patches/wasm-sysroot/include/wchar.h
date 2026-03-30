// wchar.h for wasm32-unknown-unknown
#ifndef _WCHAR_H
#define _WCHAR_H

#include <stddef.h>
#include <stdarg.h>

#ifndef _WINT_T_DEFINED
#define _WINT_T_DEFINED
typedef unsigned int wint_t;
#endif

#ifndef _WCHAR_T_DEFINED
#define _WCHAR_T_DEFINED
#ifndef __cplusplus
typedef int wchar_t;
#endif
#endif

#define WEOF ((wint_t)-1)

typedef struct { int __state; } mbstate_t;

struct FILE;

// String operations
size_t wcslen(const wchar_t *);
wchar_t *wcscpy(wchar_t *, const wchar_t *);
wchar_t *wcsncpy(wchar_t *, const wchar_t *, size_t);
wchar_t *wcscat(wchar_t *, const wchar_t *);
wchar_t *wcsncat(wchar_t *, const wchar_t *, size_t);
int wcscmp(const wchar_t *, const wchar_t *);
int wcsncmp(const wchar_t *, const wchar_t *, size_t);
wchar_t *wcschr(const wchar_t *, wchar_t);
wchar_t *wcsrchr(const wchar_t *, wchar_t);
wchar_t *wcsstr(const wchar_t *, const wchar_t *);
size_t wcsspn(const wchar_t *, const wchar_t *);
size_t wcscspn(const wchar_t *, const wchar_t *);
wchar_t *wcspbrk(const wchar_t *, const wchar_t *);
wchar_t *wcstok(wchar_t *, const wchar_t *, wchar_t **);

// Memory operations
wchar_t *wmemcpy(wchar_t *, const wchar_t *, size_t);
wchar_t *wmemmove(wchar_t *, const wchar_t *, size_t);
wchar_t *wmemset(wchar_t *, wchar_t, size_t);
int wmemcmp(const wchar_t *, const wchar_t *, size_t);
wchar_t *wmemchr(const wchar_t *, wchar_t, size_t);

// Conversion
long wcstol(const wchar_t *, wchar_t **, int);
unsigned long wcstoul(const wchar_t *, wchar_t **, int);
double wcstod(const wchar_t *, wchar_t **);

// Multibyte
int mbtowc(wchar_t *, const char *, size_t);
int wctomb(char *, wchar_t);
size_t mbrtowc(wchar_t *, const char *, size_t, mbstate_t *);
size_t wcrtomb(char *, wchar_t, mbstate_t *);
size_t mbsrtowcs(wchar_t *, const char **, size_t, mbstate_t *);
size_t wcsrtombs(char *, const wchar_t **, size_t, mbstate_t *);

// I/O
int fwide(struct FILE *, int);
int wprintf(const wchar_t *, ...);
int fwprintf(struct FILE *, const wchar_t *, ...);
int swprintf(wchar_t *, size_t, const wchar_t *, ...);
int vwprintf(const wchar_t *, va_list);
int vfwprintf(struct FILE *, const wchar_t *, va_list);
int vswprintf(wchar_t *, size_t, const wchar_t *, va_list);

wint_t btowc(int);
int wctob(wint_t);

#endif
