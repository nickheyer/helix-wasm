// Minimal assert.h for wasm32-unknown-unknown
#ifndef _ASSERT_H
#define _ASSERT_H

void abort(void);

#ifdef NDEBUG
#define assert(x) ((void)0)
#else
#define assert(x) ((x) ? (void)0 : abort())
#endif

#endif
