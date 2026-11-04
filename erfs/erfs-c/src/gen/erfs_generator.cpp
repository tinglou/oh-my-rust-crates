#include "erfs_generator.h"

#include <filesystem>
#include <fstream>
#include <iostream>
#include <vector>
#include <memory>
#include <algorithm>

#define ERFS_MAX_SIZE                (1024 * 1024 * 100)

#define ERFS_GENERATED_PREFIX       "erfs_gen_"


/// ================== copy  from resource.h =========================
///
/// ERFSEntry flags
///
enum ErfsEntryFlags {
    ERFS_DIRECTORY       = 1,
};
/// ================== copy  from resource.h =========================

namespace fs = std::filesystem;

class RfsGenEntry {
private:
    std::string name_;
    fs::path    path_;
    uint32_t    flags_;
    
    int         ordinal_;
    int         name_offset_;

    int         data_offset_;
    int         size_;
public:
    virtual ~RfsGenEntry() {};
    const auto& name() {return name_;}
    auto& name(const std::string& name){
        this->name_ = name;
        return *this;
    }

    const auto& path() {return path_;}
    auto& path(const fs::path& path){
        this->path_ = path;
        return *this;
    }

    auto flags(){return flags_;}
    auto& flags(uint32_t flags){this->flags_ = flags; return *this;}

    auto& size(int size) {this->size_ = size; return *this;}
    auto size(){return size_;};

    auto ordinal() {return ordinal_;};
    auto& ordinal(int ordinal) {this->ordinal_ = ordinal; return *this;}

    auto name_offset(){return name_offset_;};
    auto& name_offset(int name_offset) {this->name_offset_ = name_offset; return *this;}
    
    auto data_offset(){return data_offset_;};
    auto& data_offset(int data_offset) {this->data_offset_ = data_offset; return *this;}

    virtual void debug(int indent = 0) {
        for (int i = 0; i < indent; i++) {
            std::cout << "    ";
        }
        std::cout << name() << ", size: " << size() << std::endl;
    }
    virtual bool is_directory() = 0;
};

class RfsGenDirectory:public RfsGenEntry{
private:
    std::vector<std::shared_ptr<RfsGenEntry> > entries_;
public:
    auto& entries(){return entries_;};

    virtual void debug(int indent = 0) override{
        RfsGenEntry::debug(indent);

        for (auto& en : entries()) {
            en ->debug(indent + 1);
        }
    }

    virtual bool is_directory() override {return true;};
};

class RfsGenFile:public RfsGenEntry{
public:
    virtual bool is_directory() override {return false;};
};

static int build_tree(std::shared_ptr<RfsGenDirectory>& dir);
static int generate_source (std::ostream& os, std::shared_ptr<RfsGenDirectory>& dir, const std::string& id, int options);
static int generate_header (std::ostream& os, const std::string& id);
static int generate_rust (std::ostream& os, const std::string& id);

enum RfsGenTravelType {
    ERFS_GEN_TRAVEL_DIR_ENTER,
    ERFS_GEN_TRAVEL_DIR_LEAVE,
    ERFS_GEN_TRAVEL_ENTRY,
};
typedef int (*rfsgen_visit) (std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx);

static int callback_data_entry_name(std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx);
static int callback_data_file_content (std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx);
static int callback_directory_entry (std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx);

///
/// generate ERFS .c source file
///@param path the directory or file to be embedded
///@param id identity of the FS, format: [a-z][a-z_0-9]*
///@param option e.g. generate rust binding
///@param target_dir target directory 
int erfs_generate(const char *path, const char *id, int options, const char *target_dir) {
    int result = 0;

    //
    // pahse 0: check input parameters
    //
    fs::path source(path);
    if (!fs::exists(source) || !(fs::is_directory(source) || fs::is_regular_file(source))) {
        return ERFS_NOT_FOUND;
    }
    
    fs::path target(target_dir);
    if (!fs::exists(target) || !fs::is_directory(target)) {
        return ERFS_TARGET_NOT_EXIST;
    }
    
    //
    // phase 1: build the directory tree
    //
    auto root = std::make_shared<RfsGenDirectory>();

    root->path(source).name("/");
    if (!(fs::is_directory(source))) {
        auto file = std::make_shared<RfsGenFile>();
        file->path(source).name(source.filename());

        root->path(source.parent_path());
        root->entries().push_back(file);
    } else {
        result = build_tree(root);
    }

    //
    // phase 2: generate the ERFS source file
    //
    {
        // .c source file
        std::string name = std::string("erfs_") + std::string(id) + std::string(".c");
        fs::path rfsfile = target / name;
        std::ofstream ofs(rfsfile);
        std::cout << "Packaging: " << source << " to " << rfsfile << std::endl;
        generate_source(ofs, root, id, options);
    }
    {
        // .h header file
        std::string name = std::string("erfs_") + std::string(id) + std::string(".h");
        fs::path rfsfile = target / name;
        std::ofstream ofs(rfsfile);
        std::cout << "Generating header: " << rfsfile << std::endl;
        generate_header(ofs, id);
    }
    if ((options & ERFS_GEN_RUST) != 0){
        // rust(.rs) file
        std::string name = std::string("erfs_") + std::string(id) + std::string(".rs");
        fs::path rfsfile = target / name;
        std::ofstream ofs(rfsfile);
        std::cout << "Generating Ruet: " << rfsfile << std::endl;
        generate_rust(ofs, id);
    }

    return 0;
}


///
/// preprocess the directory tree
///
static int build_tree(std::shared_ptr<RfsGenDirectory>& dir) {
    int result = 0;
    for(auto& p: fs::directory_iterator(dir->path())) {
        auto path = p.path();

        // std::cout << p.path() << std::endl;
        if (fs::is_directory(path)) {
            auto subdir = std::make_shared<RfsGenDirectory>();
            subdir->name(path.filename()).path(path);
            result = build_tree(subdir);
            dir->entries().push_back(subdir);
        } else {
            auto file = std::make_shared<RfsGenFile>();
            file->name(path.filename()).path(path);
            dir->entries().push_back(file);
        }
    }

    // sort the entries
    std::sort(dir->entries().begin(), dir->entries().end(), [](const auto& left, const auto& right){
        return left->name() < (right->name());
    });
    
    return result;
}



static int rfsgen_travel_tree(std::shared_ptr<RfsGenEntry>& entry, rfsgen_visit callback, void* ctx) {
    int result = 0;

    if (entry->is_directory()) {
        result = (*callback)(entry, ERFS_GEN_TRAVEL_DIR_ENTER, ctx);
        if (result) {
            return result;
        }

        // directory entries
        std::shared_ptr<RfsGenDirectory> dir = std::dynamic_pointer_cast<RfsGenDirectory> (entry);
        // pass 1: all entries
        for (auto& en: dir->entries()) {
            result = (*callback)(en, ERFS_GEN_TRAVEL_ENTRY, ctx);
            if (result) {
                return result;
            }
        }
        // pass 2: subdirectory
        for (auto& en: dir->entries()) {
            if(en->is_directory()) {
                result = rfsgen_travel_tree(en, callback, ctx);
                if (result) {
                    return result;
                }
            }
        }

        result = (*callback)(entry, ERFS_GEN_TRAVEL_DIR_LEAVE, ctx);
        if (result) {
            return result;
        }
    } else {
        return -1;
    }
    return result;
}


struct CodegenContext {
    // config
    std::ostream& os;
    
    // state, updated by the callback functions.
    int ordinal;
    int offset;
    int escape;
    bool first;
};

static int print_license(std::ostream& os) {
    os  << "/**" << std::endl
        << " automatically generated by erfs_gen." << std::endl
        << " https://github.com/tinglou/erfs ." << std::endl
        << "*/" << std::endl
        << std::endl;
    return 0;
}

static int generate_header (std::ostream& os, const std::string& id) {
    print_license(os);
    os  << "#pragma once" << std::endl
        << std::endl
        << "#if defined(__ERFS_IMPL__)" << std::endl
        << "#include \"resource_fs.h\"" << std::endl
        << "#else // defined(__ERFS_IMPL__)" << std::endl
        << "typedef const void* ErfsRoot;" << std::endl
        << "#endif // defined(__ERFS_IMPL__)" << std::endl
        << std::endl;

    os  << "#if defined(__cplusplus)" << std::endl
        << "extern \"C\" {" << std::endl
        << "#endif" << std::endl
        << std::endl;

    os  << "/**" << std::endl
        << "* Get the embedded resource FS instance." << std::endl
        << "*/" << std::endl
        << "ErfsRoot " ERFS_GENERATED_PREFIX << id << "();" << std::endl
        << std::endl;

    os  << "#if defined(__cplusplus)" << std::endl
        << "}" << std::endl
        << "#endif" << std::endl
        << std::endl;

    return 0;
}

static int generate_rust (std::ostream& os, const std::string& id) {
    print_license(os);
    os  << "pub type ErfsRoot = *const ::std::os::raw::c_void;" << std::endl
        << std::endl
        << "extern \"C\" {" << std::endl
        << "  fn " ERFS_GENERATED_PREFIX << id << "() -> ErfsRoot;" << std::endl
        << "}" << std::endl
        << std::endl

        << "pub fn erfs_root() -> ErfsRoot {" << std::endl
        << "  unsafe {" << std::endl
        << "    " ERFS_GENERATED_PREFIX << id << "()" << std::endl
        << "  }" << std::endl
        << "}" << std::endl
        << std::endl
        ;

    return 0;
}

static int generate_source (std::ostream& os, std::shared_ptr<RfsGenDirectory>& dir, const std::string& id, int options) {
    print_license(os);
    os  << "#define  __ERFS_IMPL__" << std::endl
        << "#include \"erfs_" << id << ".h\"" << std::endl
        << std::endl

        << "static const ErfsFileSystem " ERFS_GENERATED_PREFIX << id << "_;" << std::endl
        << "ErfsRoot " ERFS_GENERATED_PREFIX << id << "(){" << std::endl
        << "  return (ErfsRoot)&" ERFS_GENERATED_PREFIX << id << "_;" << std::endl
        << "}" << std::endl
        << std::endl
        
        << "static const ErfsFileSystem " ERFS_GENERATED_PREFIX << id << "_ = {" << std::endl;


    // 
    // The .data section has 2 parts:
    // 1. directory and file names 
    // 2. file contents
    //
    os << "  .data = (uint8_t *)" << std::endl;
    CodegenContext ctx = {os, 0, 0, 0, true};
    std::shared_ptr<RfsGenEntry> entry = std::dynamic_pointer_cast<RfsGenEntry> (dir);
    
    os << "  // entry names" << std::endl;
    callback_data_entry_name(entry, ERFS_GEN_TRAVEL_ENTRY, &ctx);
    rfsgen_travel_tree(entry, callback_data_entry_name, &ctx);

    os << "  // file contents" << std::endl;
    rfsgen_travel_tree(entry, callback_data_file_content, &ctx);

    // 
    // .data_size
    //
    os  << "  ," << std::endl;

    os  << "  // data_size" << std::endl
        << "  .data_size = " << ctx.offset;

    // 
    // .entry_count
    //
    os  << "," << std::endl;
    os << "  // entry_count" << std::endl
        << "  .entry_count = " << ctx.ordinal;

    // 
    // .entries
    //
    os  << "," << std::endl;
    os << "  // directory tree: {name_offset, name_length, data_offset, data_size, flags}" << std::endl;
    os << "  .entries = (ErfsEntry[]){" << std::endl;
    callback_directory_entry(entry, ERFS_GEN_TRAVEL_ENTRY, &ctx);
    rfsgen_travel_tree(entry, callback_directory_entry, &ctx);
    os << std::endl << "  }";

    os  << std::endl;
    os  << "};" << std::endl;
    return 0;
}


/// https://en.cppreference.com/w/cpp/string/byte/isprint
/// https://en.cppreference.com/w/cpp/language/string_literal
/// https://en.cppreference.com/w/cpp/language/escape
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
            // isprint
            std::string str;
            str.push_back(ch);
            return str;
        } else {
            char tmp[16];
            sprintf(tmp, "\\x%02x", (ch & 0xFF));
            return tmp; 
        }
    }
    return "";
}

static void output_line(std::ostream& os, const uint8_t* buf, int len, CodegenContext &ctx) {
    os << "\"";
    for (int i = 0; i < len; i++) {
        auto str = escape_char(buf[i]);
        int cur_escape = (str.length() < 4)? 0 : 1;
        if ((ctx.escape ^ cur_escape) != 0) {
            ctx.escape ^= 1;
            os << "\" \"";
        }
        os << str;
    }
    os << "\"" << std::endl;
}

static int callback_data_entry_name(std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx) {
    if (ERFS_GEN_TRAVEL_ENTRY == type) {
        CodegenContext* c = reinterpret_cast<CodegenContext*>(ctx);
        entry->ordinal(c->ordinal);
        entry->name_offset(c->offset);
        
        c->ordinal++;
        c->offset += entry->name().length();
        
        const char* t = entry->is_directory() ? "D" : "F";
        c->os << "    // " << t << "[" << entry->ordinal() << "]: "  << entry->path() << std::endl;
        c->os << "  "; 
        output_line(c->os, (uint8_t*)(entry->name().c_str()), entry->name().length(), *c); 
    }
    return 0;
}

static int callback_data_file_content (std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx) {
    if (ERFS_GEN_TRAVEL_ENTRY != type || entry->is_directory()) {
        return 0;
    }

    CodegenContext* c = reinterpret_cast<CodegenContext*>(ctx);
    c->os << "  // [" << entry->ordinal() << "]: "  << entry->path() << std::endl;

    fs::path source = entry->path();

    entry->data_offset(c->offset);
    entry->size(fs::file_size(source));
    c->offset += entry->size();

    std::ifstream ifs(source, std::ios::binary);
    unsigned char buf[128];
    int len;
    while (true) {
        ifs.read(reinterpret_cast<char*>(buf), 80);
        len = ifs.gcount();
        if(len <= 0) {
            break;
        }
        c->os << "  ";
        output_line(c->os, buf, len, *c);
    }
    return 0;
}

static int callback_directory_entry (std::shared_ptr<RfsGenEntry>& entry, enum RfsGenTravelType type, void* ctx) {
    if (ERFS_GEN_TRAVEL_ENTRY != type) {
        return 0;
    }

    CodegenContext* c = reinterpret_cast<CodegenContext*>(ctx);
    if(c->first) {
        c->first = false;
    } else {
        c->os << "," << std::endl;
    }

    if(entry->is_directory()) {
        c->os << "    // [" << entry->ordinal() << "]: "  << entry->path() << std::endl
            << "    {"
            // name
            <<  entry->name_offset() << ", " << entry->name().length() ;
        // directory entries
        auto dir = std::dynamic_pointer_cast<RfsGenDirectory>(entry);
        if (dir->entries().size() > 0) {
            c->os << ", " << dir->entries()[0]->ordinal() << ", " << dir->entries().size();
        } else {
            c->os << ", 0, 0";
        }
        // FLAGS
        c->os  << ", ERFS_DIRECTORY";
        c->os  << "}";
    } else {
        c->os  << "    // [" << entry->ordinal() << "]: " << entry->path() << std::endl
            << "    {"
            // name
            << entry->name_offset() << ", " << entry->name().length()
            // content
            << ", " << entry->data_offset() << ", " << entry->size();
        // flags - always 0 for files (no gzip support)
        c->os<< ", 0";
        c->os<< "}";
    }
    return 0;
}