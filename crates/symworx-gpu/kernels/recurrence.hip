// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.
//
// Packed Euclidean recurrence mask. One thread owns one output byte
// (up to eight column pairs) so the write does not need an atomic.
//
// Compiled by hipcc. HIP_PLATFORM=amd or HIP_PLATFORM=nvidia.

#include <hip/hip_runtime.h>

#include <cstddef>

namespace {

__global__ void recurrence_kernel(
    const double* __restrict__ left,
    const double* __restrict__ right,
    int nx,
    int ny,
    int dim,
    double radius2,
    int theiler,
    unsigned char* __restrict__ bits,
    int stride) {
    const int byte_j = static_cast<int>(blockIdx.x * blockDim.x + threadIdx.x);
    const int i = static_cast<int>(blockIdx.y * blockDim.y + threadIdx.y);
    if (i >= nx || byte_j >= stride) {
        return;
    }

    const int j0 = byte_j * 8;
    unsigned char packed = 0;
    const double* row_i = left + static_cast<std::size_t>(i) * static_cast<std::size_t>(dim);

    for (int t = 0; t < 8; ++t) {
        const int j = j0 + t;
        if (j >= ny) {
            break;
        }
        const long long apart = static_cast<long long>(i) - static_cast<long long>(j);
        const long long dist_ij = apart < 0 ? -apart : apart;
        if (dist_ij <= static_cast<long long>(theiler)) {
            continue;
        }

        const double* row_j = right + static_cast<std::size_t>(j) * static_cast<std::size_t>(dim);
        double acc = 0.0;
        for (int k = 0; k < dim; ++k) {
            const double d = row_i[k] - row_j[k];
            acc += d * d;
        }
        if (acc <= radius2) {
            packed = static_cast<unsigned char>(packed | (1u << t));
        }
    }

    bits[static_cast<std::size_t>(i) * static_cast<std::size_t>(stride) + static_cast<std::size_t>(byte_j)] =
        packed;
}

int launch_recurrence(
    const double* d_left,
    const double* d_right,
    int nx,
    int ny,
    int dim,
    double radius2,
    int theiler,
    unsigned char* d_bits,
    int stride) {
    const dim3 block(16, 16);
    const dim3 grid(
        static_cast<unsigned int>((stride + 15) / 16),
        static_cast<unsigned int>((nx + 15) / 16));
    hipLaunchKernelGGL(
        recurrence_kernel,
        grid,
        block,
        0,
        0,
        d_left,
        d_right,
        nx,
        ny,
        dim,
        radius2,
        theiler,
        d_bits,
        stride);
    hipError_t st = hipGetLastError();
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    st = hipDeviceSynchronize();
    return static_cast<int>(st);
}

}  // namespace

extern "C" int symworx_hip_device_count(int* count) {
    if (count == nullptr) {
        return -2;
    }
    int n = 0;
    const hipError_t st = hipGetDeviceCount(&n);
    if (st != hipSuccess) {
        *count = 0;
        return static_cast<int>(st);
    }
    *count = n;
    return 0;
}

extern "C" int symworx_hip_device_name(int device, char* name, std::size_t len) {
    if (name == nullptr || len == 0) {
        return -2;
    }
    name[0] = '\0';
    hipError_t st = hipSetDevice(device);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    hipDeviceProp_t prop{};
    st = hipGetDeviceProperties(&prop, device);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    std::size_t i = 0;
    for (; i + 1 < len && prop.name[i] != '\0'; ++i) {
        name[i] = prop.name[i];
    }
    name[i] = '\0';
    return 0;
}

extern "C" int symworx_recurrence_f64(
    const double* left,
    int nx,
    const double* right,
    int ny,
    int dim,
    double radius2,
    int theiler,
    int same,
    int device,
    unsigned char* host_bits,
    int bits_len) {
    if (nx < 0 || ny < 0 || dim <= 0 || bits_len < 0 || theiler < 0 || device < 0) {
        return -1;
    }
    if (nx == 0 || ny == 0) {
        return 0;
    }
    if (left == nullptr || host_bits == nullptr || (same == 0 && right == nullptr)) {
        return -2;
    }

    const int stride = (ny + 7) / 8;
    const std::size_t bit_bytes = static_cast<std::size_t>(nx) * static_cast<std::size_t>(stride);
    if (bit_bytes != static_cast<std::size_t>(bits_len)) {
        return -3;
    }

    hipError_t st = hipSetDevice(device);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }

    const std::size_t left_bytes =
        static_cast<std::size_t>(nx) * static_cast<std::size_t>(dim) * sizeof(double);
    const std::size_t right_bytes =
        static_cast<std::size_t>(ny) * static_cast<std::size_t>(dim) * sizeof(double);

    double* d_left = nullptr;
    double* d_right = nullptr;
    unsigned char* d_bits = nullptr;

    st = hipMalloc(reinterpret_cast<void**>(&d_left), left_bytes);
    if (st != hipSuccess) {
        return static_cast<int>(st);
    }
    st = hipMemcpy(d_left, left, left_bytes, hipMemcpyHostToDevice);
    if (st != hipSuccess) {
        hipFree(d_left);
        return static_cast<int>(st);
    }

    if (same) {
        d_right = d_left;
    } else {
        st = hipMalloc(reinterpret_cast<void**>(&d_right), right_bytes);
        if (st != hipSuccess) {
            hipFree(d_left);
            return static_cast<int>(st);
        }
        st = hipMemcpy(d_right, right, right_bytes, hipMemcpyHostToDevice);
        if (st != hipSuccess) {
            hipFree(d_right);
            hipFree(d_left);
            return static_cast<int>(st);
        }
    }

    st = hipMalloc(reinterpret_cast<void**>(&d_bits), bit_bytes);
    if (st != hipSuccess) {
        if (!same) {
            hipFree(d_right);
        }
        hipFree(d_left);
        return static_cast<int>(st);
    }

    const int launched = launch_recurrence(d_left, d_right, nx, ny, dim, radius2, theiler, d_bits, stride);
    if (launched != 0) {
        hipFree(d_bits);
        if (!same) {
            hipFree(d_right);
        }
        hipFree(d_left);
        return launched;
    }

    st = hipMemcpy(host_bits, d_bits, bit_bytes, hipMemcpyDeviceToHost);

    hipFree(d_bits);
    if (!same) {
        hipFree(d_right);
    }
    hipFree(d_left);
    return static_cast<int>(st);
}
