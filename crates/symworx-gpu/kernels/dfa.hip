// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.
//
// Order-1 DFA residual sums. One thread owns one box and writes one sum of
// squares. The host adds those sums in CPU order and takes the root mean
// square. The host chooses the box lengths. This kernel does not fit a slope
// and does not read a rate.
//
// Compiled by hipcc into the same libsymworx_hip.so as recurrence.hip.
// HIP_PLATFORM=amd or HIP_PLATFORM=nvidia.

#include <hip/hip_runtime.h>

#include <cstddef>

namespace {

// Least-squares line on t = 0 .. n-1. sum (t - mean_t)^2 = n (n^2 - 1) / 12.
__device__ double linear_residual_sum_sq(const double* segment, int n) {
    const double nd = static_cast<double>(n);
    const double mean_t = (nd - 1.0) / 2.0;
    double sum_y = 0.0;
    for (int i = 0; i < n; ++i) {
        sum_y += segment[i];
    }
    const double mean_y = sum_y / nd;
    double cov = 0.0;
    for (int i = 0; i < n; ++i) {
        cov += (static_cast<double>(i) - mean_t) * (segment[i] - mean_y);
    }
    const double sum_sq_t = nd * (nd * nd - 1.0) / 12.0;
    const double slope = cov / sum_sq_t;
    const double intercept = mean_y - slope * mean_t;
    double sum_sq = 0.0;
    for (int i = 0; i < n; ++i) {
        const double residual = segment[i] - (intercept + slope * static_cast<double>(i));
        sum_sq += residual * residual;
    }
    return sum_sq;
}

__global__ void dfa_linear_ss_kernel(
    const double* __restrict__ profile,
    const int* __restrict__ starts,
    const int* __restrict__ scales,
    double* __restrict__ ss,
    int n_boxes) {
    const int box = static_cast<int>(blockIdx.x * blockDim.x + threadIdx.x);
    if (box >= n_boxes) {
        return;
    }
    ss[box] = linear_residual_sum_sq(profile + starts[box], scales[box]);
}

int launch_dfa(const double* d_profile, const int* d_starts, const int* d_scales, double* d_ss, int n_boxes) {
    const unsigned int block = 256;
    const unsigned int grid = static_cast<unsigned int>((n_boxes + 255) / 256);
    hipLaunchKernelGGL(
        dfa_linear_ss_kernel,
        dim3(grid),
        dim3(block),
        0,
        0,
        d_profile,
        d_starts,
        d_scales,
        d_ss,
        n_boxes);
    hipError_t st = hipGetLastError();
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    st = hipDeviceSynchronize();
    return static_cast<int>(st);
}

}  // namespace

// Per-box residual sums for order-1 DFA.
//
// `starts[i]` is the first profile index of box i. `scales[i]` is its length.
// `host_ss` receives one sum of squares per box, in the same order.
// Returns 0 on success, a negative launcher code, or a HIP status.
extern "C" int symworx_dfa_linear_ss_f64(
    const double* profile,
    int n,
    const int* starts,
    const int* scales,
    int n_boxes,
    int device,
    double* host_ss) {
    if (n <= 0 || n_boxes < 0 || device < 0) {
        return -1;
    }
    if (n_boxes == 0) {
        return 0;
    }
    if (profile == nullptr || starts == nullptr || scales == nullptr || host_ss == nullptr) {
        return -2;
    }
    for (int i = 0; i < n_boxes; ++i) {
        const int scale = scales[i];
        const int start = starts[i];
        if (scale < 3 || start < 0 || start > n - scale) {
            return -1;
        }
    }

    hipError_t st = hipSetDevice(device);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }

    const std::size_t profile_bytes = static_cast<std::size_t>(n) * sizeof(double);
    const std::size_t index_bytes = static_cast<std::size_t>(n_boxes) * sizeof(int);
    const std::size_t ss_bytes = static_cast<std::size_t>(n_boxes) * sizeof(double);

    double* d_profile = nullptr;
    int* d_starts = nullptr;
    int* d_scales = nullptr;
    double* d_ss = nullptr;

    st = hipMalloc(reinterpret_cast<void**>(&d_profile), profile_bytes);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    st = hipMemcpy(d_profile, profile, profile_bytes, hipMemcpyHostToDevice);
    if (st != hipSuccess) {
        hipFree(d_profile);
        return static_cast<int>(st);
    }

    st = hipMalloc(reinterpret_cast<void**>(&d_starts), index_bytes);
    if (st != hipSuccess) {
        hipFree(d_profile);
        return static_cast<int>(st);
    }
    st = hipMemcpy(d_starts, starts, index_bytes, hipMemcpyHostToDevice);
    if (st != hipSuccess) {
        hipFree(d_starts);
        hipFree(d_profile);
        return static_cast<int>(st);
    }

    st = hipMalloc(reinterpret_cast<void**>(&d_scales), index_bytes);
    if (st != hipSuccess) {
        hipFree(d_starts);
        hipFree(d_profile);
        return static_cast<int>(st);
    }
    st = hipMemcpy(d_scales, scales, index_bytes, hipMemcpyHostToDevice);
    if (st != hipSuccess) {
        hipFree(d_scales);
        hipFree(d_starts);
        hipFree(d_profile);
        return static_cast<int>(st);
    }

    st = hipMalloc(reinterpret_cast<void**>(&d_ss), ss_bytes);
    if (st != hipSuccess) {
        hipFree(d_scales);
        hipFree(d_starts);
        hipFree(d_profile);
        return static_cast<int>(st);
    }

    const int launched = launch_dfa(d_profile, d_starts, d_scales, d_ss, n_boxes);
    if (launched != 0) {
        hipFree(d_ss);
        hipFree(d_scales);
        hipFree(d_starts);
        hipFree(d_profile);
        return launched;
    }

    st = hipMemcpy(host_ss, d_ss, ss_bytes, hipMemcpyDeviceToHost);
    hipFree(d_ss);
    hipFree(d_scales);
    hipFree(d_starts);
    hipFree(d_profile);
    return static_cast<int>(st);
}
