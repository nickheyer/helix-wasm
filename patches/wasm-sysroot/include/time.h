#ifndef _TIME_H
#define _TIME_H
#include <stddef.h>
typedef long time_t;
typedef long clock_t;
struct timespec { time_t tv_sec; long tv_nsec; };
int clock_gettime(int, struct timespec *);
#define CLOCK_MONOTONIC 1
#define CLOCK_REALTIME 0
#endif
