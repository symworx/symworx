// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.
//
// Sample-entropy pair counts. One thread walks template rows i < j.
// A pair matches at m unless some lag has abs(x[i+k] - x[j+k]) > r.
// The m+1 count also requires abs(x[i+m] - x[j+m]) <= r.
//
// Compiled by hipcc into the same libsymworx_hip.so as recurrence.hip.
// HIP_PLATFORM=amd or HIP_PLATFORM=nvidia.

#include <hip/hip_runtime.h>

#include <cstddef>

namespace {

constexpr int BLOCK = 256;

__device__ unsigned long long reduce_block(unsigned long long value) {
    __shared__ unsigned long long buf[BLOCK];
    buf[threadIdx.x] = value;
    __syncthreads();
    for (int stride = BLOCK / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            buf[threadIdx.x] += buf[threadIdx.x + stride];
        }
        __syncthreads();
    }
    return buf[0];
}

__global__ void sample_entropy_kernel(
    const double* __restrict__ data,
    int templates,
    int m,
    double r,
    unsigned long long* __restrict__ count_m,
    unsigned long long* __restrict__ count_m1) {
    unsigned long long local_m = 0;
    unsigned long long local_m1 = 0;
    const int step = static_cast<int>(blockDim.x * gridDim.x);

    for (int i = static_cast<int>(blockIdx.x * blockDim.x + threadIdx.x); i < templates - 1; i += step) {
        for (int j = i + 1; j < templates; ++j) {
            bool match_m = true;
            for (int k = 0; k < m; ++k) {
                const double diff = fabs(data[i + k] - data[j + k]);
                if (diff > r) {
                    match_m = false;
                    break;
                }
            }
            if (!match_m) {
                continue;
            }
            local_m += 1;
            const double next = fabs(data[i + m] - data[j + m]);
            if (next <= r) {
                local_m1 += 1;
            }
        }
    }

    const unsigned long long block_m = reduce_block(local_m);
    const unsigned long long block_m1 = reduce_block(local_m1);
    if (threadIdx.x == 0) {
        atomicAdd(count_m, block_m);
        atomicAdd(count_m1, block_m1);
    }
}

int launch_sample_entropy(
    const double* d_data,
    int templates,
    int m,
    double r,
    unsigned long long* d_count_m,
    unsigned long long* d_count_m1) {
    const int rows = templates - 1;
    unsigned int grid = static_cast<unsigned int>((rows + BLOCK - 1) / BLOCK);
    if (grid > 1024u) {
        grid = 1024u;
    }
    if (grid == 0u) {
        grid = 1u;
    }
    hipLaunchKernelGGL(
        sample_entropy_kernel,
        dim3(grid),
        dim3(BLOCK),
        0,
        0,
        d_data,
        templates,
        m,
        r,
        d_count_m,
        d_count_m1);
    hipError_t st = hipGetLastError();
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    st = hipDeviceSynchronize();
    return static_cast<int>(st);
}

}  // namespace

extern "C" int symworx_sample_entropy_f64(
    const double* data,
    int n,
    int m,
    double r,
    int device,
    unsigned long long* count_m,
    unsigned long long* count_m1) {
    if (count_m == nullptr || count_m1 == nullptr) {
        return -2;
    }
    *count_m = 0;
    *count_m1 = 0;
    if (n < 0 || m < 0 || device < 0 || m > n) {
        return -1;
    }
    const int templates = n - m;
    if (templates < 2) {
        return 0;
    }
    if (data == nullptr) {
        return -2;
    }

    hipError_t st = hipSetDevice(device);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }

    const std::size_t bytes = static_cast<std::size_t>(n) * sizeof(double);
    double* d_data = nullptr;
    unsigned long long* d_counts = nullptr;

    st = hipMalloc(reinterpret_cast<void**>(&d_data), bytes);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    st = hipMemcpy(d_data, data, bytes, hipMemcpyHostToDevice);
    if (st != hipSuccess) {
        hipFree(d_data);
        return static_cast<int>(st);
    }

    st = hipMalloc(reinterpret_cast<void**>(&d_counts), 2 * sizeof(unsigned long long));
    if (st != hipSuccess) {
        hipFree(d_data);
        return static_cast<int>(st);
    }
    st = hipMemset(d_counts, 0, 2 * sizeof(unsigned long long));
    if (st != hipSuccess) {
        hipFree(d_counts);
        hipFree(d_data);
        return static_cast<int>(st);
    }

    const int launched = launch_sample_entropy(d_data, templates, m, r, d_counts, d_counts + 1);
    if (launched != 0) {
        hipFree(d_counts);
        hipFree(d_data);
        return launched;
    }

    unsigned long long host_counts[2] = {0, 0};
    st = hipMemcpy(host_counts, d_counts, 2 * sizeof(unsigned long long), hipMemcpyDeviceToHost);
    hipFree(d_counts);
    hipFree(d_data);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    *count_m = host_counts[0];
    *count_m1 = host_counts[1];
    return 0;
}
