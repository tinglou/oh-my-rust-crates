#include "erfs_generator.h"

#include <filesystem>
#include <fstream>
#include <iostream>
#include <vector>
#include <memory>
#include <algorithm>
#include <cstring>
#include <cstdint>

#define ERFS_MAGIC 0x45524653
#define ERFS_HEADER_SIZE 12
#define ERFS_ENTRY_SIZE 16

namespace fs = std::filesystem;

struct FlatEntry {
    uint32_t data_offset;
    uint32_t data_size;
    uint32_t name_offset;
    uint16_t name_size;
    uint16_t flags;
    std::string name;
    fs::path path;
    bool is_dir;
};

class RfsGenEntry {
public:
    std::string name_;
    fs::path    path_;
    virtual ~RfsGenEntry() {}
    virtual bool is_directory() = 0;
};

class RfsGenDirectory : public RfsGenEntry {
public:
    std::vector<std::shared_ptr<RfsGenEntry>> entries_;
    virtual bool is_directory() override { return true; }
};

class RfsGenFile : public RfsGenEntry {
public:
    virtual bool is_directory() override { return false; }
};

static int build_tree(std::shared_ptr<RfsGenDirectory>& dir);
static void flatten_directory(std::shared_ptr<RfsGenDirectory>& dir, std::vector<FlatEntry>& flat, int dir_ordinal);
static std::vector<uint8_t> build_image(std::vector<FlatEntry>& entries);
static int generate_source(std::ostream& os, const std::string& id, const std::vector<uint8_t>& image);
static int generate_header(std::ostream& os, const std::string& id);
static int generate_image_file(const std::string& path, const std::vector<uint8_t>& image);

static void write_u16(std::vector<uint8_t>& buf, uint16_t val) {
    buf.push_back(val & 0xFF);
    buf.push_back((val >> 8) & 0xFF);
}

static void write_u32(std::vector<uint8_t>& buf, uint32_t val) {
    buf.push_back(val & 0xFF);
    buf.push_back((val >> 8) & 0xFF);
    buf.push_back((val >> 16) & 0xFF);
    buf.push_back((val >> 24) & 0xFF);
}

static void write_entry(std::vector<uint8_t>& buf, const FlatEntry& e) {
    write_u32(buf, e.data_offset);
    write_u32(buf, e.data_size);
    write_u32(buf, e.name_offset);
    write_u16(buf, e.name_size);
    write_u16(buf, e.flags);
}

int erfs_generate(const char *path, const char *id, int options, const char *target_dir) {
    fs::path source(path);
    if (!fs::exists(source) || !(fs::is_directory(source) || fs::is_regular_file(source))) {
        return ERFS_NOT_FOUND;
    }

    fs::path target(target_dir);
    if (!fs::exists(target) || !fs::is_directory(target)) {
        return ERFS_TARGET_NOT_EXIST;
    }

    auto root = std::make_shared<RfsGenDirectory>();
    root->path_ = source;
    root->name_ = "/";

    if (!fs::is_directory(source)) {
        auto file = std::make_shared<RfsGenFile>();
        file->path_ = source;
        file->name_ = source.filename().string();
        root->path_ = source.parent_path();
        root->entries_.push_back(file);
    } else {
        int result = build_tree(root);
        if (result != 0) return result;
    }

    std::vector<FlatEntry> flat_entries;
    FlatEntry root_fe;
    root_fe.name = root->name_;
    root_fe.path = root->path_;
    root_fe.is_dir = true;
    root_fe.name_size = (uint16_t)root->name_.size();
    root_fe.flags = 1;
    root_fe.data_offset = 0;
    root_fe.data_size = 0;
    root_fe.name_offset = 0;
    flat_entries.push_back(root_fe);

    flatten_directory(root, flat_entries, 0);

    std::vector<uint8_t> image = build_image(flat_entries);

    {
        std::string name = std::string("erfs_") + std::string(id) + std::string(".c");
        fs::path rfsfile = target / name;
        std::ofstream ofs(rfsfile);
        std::cout << "Packaging: " << source << " to " << rfsfile << std::endl;
        generate_source(ofs, id, image);
    }
    {
        std::string name = std::string("erfs_") + std::string(id) + std::string(".h");
        fs::path rfsfile = target / name;
        std::ofstream ofs(rfsfile);
        std::cout << "Generating header: " << rfsfile << std::endl;
        generate_header(ofs, id);
    }
    if ((options & ERFS_GEN_IMAGE) != 0) {
        std::string name = std::string("erfs_") + std::string(id) + std::string(".img");
        fs::path rfsfile = target / name;
        std::cout << "Generating image: " << rfsfile << std::endl;
        generate_image_file(rfsfile.string(), image);
    }

    return 0;
}

static int build_tree(std::shared_ptr<RfsGenDirectory>& dir) {
    for (auto& p : fs::directory_iterator(dir->path_)) {
        auto path = p.path();
        if (fs::is_directory(path)) {
            auto subdir = std::make_shared<RfsGenDirectory>();
            subdir->name_ = path.filename().string();
            subdir->path_ = path;
            int result = build_tree(subdir);
            if (result != 0) return result;
            dir->entries_.push_back(subdir);
        } else {
            auto file = std::make_shared<RfsGenFile>();
            file->name_ = path.filename().string();
            file->path_ = path;
            dir->entries_.push_back(file);
        }
    }

    std::sort(dir->entries_.begin(), dir->entries_.end(), [](const auto& left, const auto& right) {
        return left->name_ < right->name_;
    });

    return 0;
}

static void flatten_directory(std::shared_ptr<RfsGenDirectory>& dir, std::vector<FlatEntry>& flat, int dir_ordinal) {
    int first_child_index = (int)flat.size();

    for (auto& child : dir->entries_) {
        FlatEntry child_fe;
        child_fe.name = child->name_;
        child_fe.path = child->path_;
        child_fe.is_dir = child->is_directory();
        child_fe.name_size = (uint16_t)child->name_.size();
        child_fe.flags = child->is_directory() ? (uint16_t)1 : (uint16_t)0;
        child_fe.data_offset = 0;
        child_fe.data_size = 0;
        child_fe.name_offset = 0;
        flat.push_back(child_fe);
    }

    flat[dir_ordinal].data_offset = (uint32_t)first_child_index * ERFS_ENTRY_SIZE;
    flat[dir_ordinal].data_size = (uint32_t)dir->entries_.size();

    for (int i = 0; i < (int)dir->entries_.size(); i++) {
        if (dir->entries_[i]->is_directory()) {
            auto subdir = std::dynamic_pointer_cast<RfsGenDirectory>(dir->entries_[i]);
            int child_ordinal = first_child_index + i;
            flatten_directory(subdir, flat, child_ordinal);
        }
    }
}

static std::vector<uint8_t> build_image(std::vector<FlatEntry>& entries) {
    std::vector<uint8_t> image;

    uint16_t entry_count = (uint16_t)entries.size();
    uint32_t entry_table_size = (uint32_t)entry_count * ERFS_ENTRY_SIZE;

    uint32_t total_name_bytes = 0;
    for (auto& e : entries) {
        total_name_bytes += e.name_size;
    }

    uint32_t names_start = entry_table_size;
    uint32_t files_start = names_start + total_name_bytes;

    uint32_t name_cursor = names_start;
    for (auto& e : entries) {
        e.name_offset = name_cursor;
        name_cursor += e.name_size;
    }

    uint32_t file_cursor = files_start;
    for (auto& e : entries) {
        if (!e.is_dir) {
            e.data_offset = file_cursor;
            uint64_t fsize = fs::file_size(e.path);
            e.data_size = (uint32_t)fsize;
            file_cursor += e.data_size;
        }
    }

    uint32_t total_data_size = file_cursor;
    uint32_t total_size = ERFS_HEADER_SIZE + total_data_size;

    write_u32(image, ERFS_MAGIC);
    write_u32(image, total_size);
    write_u16(image, entry_count);
    write_u16(image, 0);

    for (auto& e : entries) {
        write_entry(image, e);
    }

    for (auto& e : entries) {
        image.insert(image.end(), e.name.begin(), e.name.end());
    }

    for (auto& e : entries) {
        if (!e.is_dir) {
            std::ifstream ifs(e.path, std::ios::binary);
            char buf[4096];
            while (ifs.read(buf, sizeof(buf))) {
                image.insert(image.end(), buf, buf + ifs.gcount());
            }
            if (ifs.gcount() > 0) {
                image.insert(image.end(), buf, buf + ifs.gcount());
            }
        }
    }

    return image;
}

static bool is_hex_digit_char(char ch) {
    return (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F');
}

static std::string escape_char(unsigned char ch) {
    switch (ch) {
    case '\\': return "\\\\";
    case '\"': return "\\\"";
    case '\a': return "\\a";
    case '\b': return "\\b";
    case '\t': return "\\t";
    case '\n': return "\\n";
    case '\v': return "\\v";
    case '\f': return "\\f";
    case '\r': return "\\r";
    default:
        if (ch >= 32 && ch <= 126) {
            std::string str;
            str.push_back(ch);
            return str;
        } else {
            char tmp[8];
            sprintf(tmp, "\\x%02x", ch & 0xFF);
            return tmp;
        }
    }
}

static bool needs_hex_escape(unsigned char ch) {
    return (ch < 32 || ch > 126);
}

static void output_bytes_as_string(std::ostream& os, const uint8_t* data, size_t len) {
    os << "\"";
    for (size_t i = 0; i < len; i++) {
        unsigned char ch = data[i];
        
        if (needs_hex_escape(ch) && i + 1 < len) {
            unsigned char next = data[i + 1];
            if (is_hex_digit_char((char)next)) {
                os << escape_char(ch) << "\" \"";
                continue;
            }
        }
        
        os << escape_char(ch);
    }
    os << "\"" << std::endl;
}

static int generate_source(std::ostream& os, const std::string& id, const std::vector<uint8_t>& image) {
    os << "/**" << std::endl
       << " automatically generated by erfs_gen." << std::endl
       << " https://github.com/tinglou/erfs ." << std::endl
       << "*/" << std::endl
       << std::endl;

    os << "#define __ERFS_IMPL__" << std::endl
       << "#include \"erfs_" << id << ".h\"" << std::endl
       << std::endl;

    os << "static const union {" << std::endl
       << "  uint32_t _align;" << std::endl
       << "  uint8_t _data[" << image.size() << "];" << std::endl
       << "} erfs_" << id << "_raw = { ._data = {" << std::endl;

    size_t header_size = ERFS_HEADER_SIZE;

    {
        uint32_t magic;
        memcpy(&magic, &image[0], 4);
        uint32_t total_size;
        memcpy(&total_size, &image[4], 4);
        uint16_t entry_count;
        memcpy(&entry_count, &image[8], 2);

        os << "  // magic: 0x" << std::hex << magic << std::dec << std::endl;
        os << "  // total_size: " << total_size << std::endl;
        os << "  // entry_count: " << entry_count << std::endl;
    }

    os << "  // header (" << header_size << " bytes)" << std::endl;
    os << "  ";
    output_bytes_as_string(os, image.data(), header_size);

    uint16_t entry_count;
    memcpy(&entry_count, &image[8], 2);

    os << "  // entry table (" << entry_count << " entries, " << entry_count * ERFS_ENTRY_SIZE << " bytes)" << std::endl;
    for (uint16_t i = 0; i < entry_count; i++) {
        size_t offset = header_size + i * ERFS_ENTRY_SIZE;
        os << "  // [" << i << "]" << std::endl;
        os << "  ";
        output_bytes_as_string(os, image.data() + offset, ERFS_ENTRY_SIZE);
    }

    size_t data_section_start = header_size + entry_count * ERFS_ENTRY_SIZE;
    size_t data_section_size = image.size() - data_section_start;

    os << "  // names + file data (" << data_section_size << " bytes)" << std::endl;

    size_t pos = data_section_start;
    size_t remaining = image.size() - pos;
    while (remaining > 0) {
        size_t len = remaining > 80 ? 80 : remaining;
        os << "  ";
        output_bytes_as_string(os, image.data() + pos, len);
        pos += len;
        remaining -= len;
    }

    os << "}};" << std::endl;
    os << std::endl;

    os << "ErfsRoot erfs_" << id << "(void) {" << std::endl
       << "  return (ErfsRoot)erfs_" << id << "_raw._data;" << std::endl
       << "}" << std::endl;

    return 0;
}

static int generate_header(std::ostream& os, const std::string& id) {
    os << "/**" << std::endl
       << " automatically generated by erfs_gen." << std::endl
       << " https://github.com/tinglou/erfs ." << std::endl
       << "*/" << std::endl
       << std::endl;

    os << "#pragma once" << std::endl
       << std::endl
       << "#if defined(__ERFS_IMPL__)" << std::endl
       << "#include \"resource_fs.h\"" << std::endl
       << "#else // defined(__ERFS_IMPL__)" << std::endl
       << "typedef const void* ErfsRoot;" << std::endl
       << "#endif // defined(__ERFS_IMPL__)" << std::endl
       << std::endl;

    os << "#if defined(__cplusplus)" << std::endl
       << "extern \"C\" {" << std::endl
       << "#endif" << std::endl
       << std::endl;

    os << "ErfsRoot erfs_" << id << "(void);" << std::endl
       << std::endl;

    os << "#if defined(__cplusplus)" << std::endl
       << "}" << std::endl
       << "#endif" << std::endl;

    return 0;
}

static int generate_image_file(const std::string& path, const std::vector<uint8_t>& image) {
    std::ofstream ofs(path, std::ios::binary);
    if (!ofs.is_open()) {
        return ERFS_TARGET_NOT_EXIST;
    }
    ofs.write(reinterpret_cast<const char*>(image.data()), image.size());
    return 0;
}
