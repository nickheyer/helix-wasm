// math.h for wasm32-unknown-unknown
#ifndef _MATH_H
#define _MATH_H

#define INFINITY __builtin_inff()
#define NAN      __builtin_nanf("")
#define HUGE_VAL __builtin_huge_val()

double fabs(double);
float fabsf(float);
double floor(double);
float floorf(float);
double ceil(double);
float ceilf(float);
double round(double);
float roundf(float);
double trunc(double);
float truncf(float);
double sqrt(double);
float sqrtf(float);
double pow(double, double);
float powf(float, float);
double log(double);
float logf(float);
double log2(double);
float log2f(float);
double log10(double);
float log10f(float);
double exp(double);
float expf(float);
double fmod(double, double);
float fmodf(float, float);
double fmin(double, double);
float fminf(float, float);
double fmax(double, double);
float fmaxf(float, float);
double sin(double);
double cos(double);
double tan(double);
double atan2(double, double);
int isnan(double);
int isinf(double);
int isfinite(double);

#endif
