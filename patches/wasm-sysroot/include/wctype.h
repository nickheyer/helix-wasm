// wctype.h for wasm32-unknown-unknown
#ifndef _WCTYPE_H
#define _WCTYPE_H

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

typedef unsigned long wctype_t;
typedef const void *wctrans_t;

#define WEOF ((wint_t)-1)

int iswalnum(wint_t);
int iswalpha(wint_t);
int iswblank(wint_t);
int iswcntrl(wint_t);
int iswdigit(wint_t);
int iswgraph(wint_t);
int iswlower(wint_t);
int iswprint(wint_t);
int iswpunct(wint_t);
int iswspace(wint_t);
int iswupper(wint_t);
int iswxdigit(wint_t);
int towlower(wint_t);
int towupper(wint_t);

wctype_t wctype(const char *);
int iswctype(wint_t, wctype_t);
wctrans_t wctrans(const char *);
wint_t towctrans(wint_t, wctrans_t);

#endif
