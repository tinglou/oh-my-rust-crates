#include "erfs_generator.h"
#include <iostream>
#include <string>
#include <cstring>

void usage(const char* prog) {
    std::cout << "Usage: " << prog << " [options] <src_dir> <id> <dest_dir>" << std::endl;
    std::cout << "Output format (default: --c):" << std::endl;
    std::cout << "  --c         generate C source file" << std::endl;
    std::cout << "  --image     generate binary image file" << std::endl;
    std::cout << "Byte order (image only, default: native):" << std::endl;
    std::cout << "  --le        generate little-endian image" << std::endl;
    std::cout << "  --be        generate big-endian image" << std::endl;
}

int main(int argc, char** argv) {
    if (argc < 4) {
        usage(argv[0]);
        return 1;
    }
    const char* real_args[3] = {0};
    int pos = 0;
    int option = 0;
    for (int i = 1; i < argc; i++) {
        char* arg = argv[i];
        if (*arg == '-') {
            if (strcmp("--c", arg) == 0) {
                option |= ERFS_GEN_C;
            } else if (strcmp("--image", arg) == 0) {
                option |= ERFS_GEN_IMAGE;
            } else if (strcmp("--le", arg) == 0) {
                option |= ERFS_GEN_LE;
            } else if (strcmp("--be", arg) == 0) {
                option |= ERFS_GEN_BE;
            } else {
                std::cout << "Unknown option: " << arg << std::endl << std::endl;
                usage(argv[0]);
                return 2;
            }
        } else {
            real_args[pos] = arg;
            pos++;
        }
    }
    if (pos != 3) {
        usage(argv[0]);
        return 3;
    }
    int result = erfs_generate(real_args[0], real_args[1], option, real_args[2]);
    return result;
}
