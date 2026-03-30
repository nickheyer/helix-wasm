// assert.h for wasm32-unknown-unknown
#ifndef _ASSERT_H
#define _ASSERT_H

void abort(void);

#ifdef NDEBUG
#define assert(x) ((void)0)
#else
#define assert(x) ((x) ? (void)0 : abort())
#endif

// C11: static_assert is a macro for _Static_assert
#ifndef static_assert
#define static_assert _Static_assert
#endif

#endif
