// time.h for wasm32-unknown-unknown
#ifndef _TIME_H
#define _TIME_H

#include <stddef.h>

typedef long time_t;
typedef long clock_t;
typedef long suseconds_t;

#define CLOCKS_PER_SEC ((clock_t)1000000)
#define CLOCK_REALTIME  0
#define CLOCK_MONOTONIC 1

struct timespec {
    time_t tv_sec;
    long   tv_nsec;
};

struct tm {
    int tm_sec;
    int tm_min;
    int tm_hour;
    int tm_mday;
    int tm_mon;
    int tm_year;
    int tm_wday;
    int tm_yday;
    int tm_isdst;
};

clock_t clock(void);
time_t time(time_t *);
double difftime(time_t, time_t);
time_t mktime(struct tm *);
struct tm *gmtime(const time_t *);
struct tm *localtime(const time_t *);
size_t strftime(char *, size_t, const char *, const struct tm *);
char *asctime(const struct tm *);
char *ctime(const time_t *);
int clock_gettime(int, struct timespec *);
int nanosleep(const struct timespec *, struct timespec *);

#endif
