// cuda/allocator.h
// Custom memory allocator wrapper to support both CUDA Unified Memory (Jetson) 
// and standard RAM allocation (macOS/CPU testing).

#pragma once

#include <stdlib.h>

#ifdef USE_CUDA
    #include <cuda_runtime.h>
    #include <stdio.h>

    // Helper macro for catching CUDA errors
    #define CUDA_CHECK(call) \
        do { \
            cudaError_t err = call; \
            if (err != cudaSuccess) { \
                fprintf(stderr, "CUDA error in %s at line %d: %s\n", \
                        __FILE__, __LINE__, cudaGetErrorString(err)); \
                exit(EXIT_FAILURE); \
            } \
        } while (0)

    template<typename T>
    T* allocate_managed(size_t count) {
        T* ptr;
        CUDA_CHECK(cudaMallocManaged(&ptr, count * sizeof(T)));
        return ptr;
    }

    template<typename T>
    void free_managed(T* ptr) {
        CUDA_CHECK(cudaFree(ptr));
    }
#else
    // macOS / CPU Mock
    template<typename T>
    T* allocate_managed(size_t count) {
        return (T*)malloc(count * sizeof(T));
    }

    template<typename T>
    void free_managed(T* ptr) {
        free(ptr);
    }
#endif
