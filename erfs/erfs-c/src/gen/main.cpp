#include "erfs_generator.h"
#include <iostream>
#include <string>
#include <cstring>

void usage(const char* prog) {
    std::cout << "Usage: " << prog << " [options] <src_dir> <id> <dest_dir>" << std::endl;
    std::cout << "Options:" << std::endl;
    std::cout << "  --image     generate raw binary image file (.img)." << std::endl;
}

int main(int argc, char** argv) {
    int result = 0;
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
            if (strcmp("--image", arg) == 0) {
                option |= ERFS_GEN_IMAGE;
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
    result = erfs_generate(real_args[0], real_args[1], option, real_args[2]);
    return result;
}
