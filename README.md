# 🧹 dirpurge

**Advanced Directory Cleanup Tool with Safety Features**

---

## 📖 Overview

`dirpurge` is a powerful and flexible command-line tool designed to efficiently clean up directories while ensuring safety and user control. With advanced filtering, interactive selection, backup options, and logging, `dirpurge` offers a robust solution for maintaining a clutter-free system.

Whether you're a developer managing multiple projects or an administrator handling large directory structures, `dirpurge` helps remove unwanted files and folders quickly and safely.

---

## 🎯 Features

✅ **Targeted Cleanup** - Specify directory names to search and remove.

🚫 **Exclusions** - Prevent deletion of specific directories.

📏 **Depth Control** - Define how deep the search should go.

📦 **Size Filtering** - Remove only directories above a certain size.

📅 **Age Filtering** - Delete directories older than a specified number of days.

🔗 **Symlink Support** - Follow symbolic links during search (optional).

🗑 **Safe Deletion** - Move files to trash instead of permanently deleting them.

💾 **Backup & Archiving** - Automatically create backups or zip archives before deletion.

🖱 **Interactive Mode** - Select directories interactively before deletion.

🔐 **Confirmation System** - Require a specific phrase to confirm deletion.

📊 **Logging & Reporting** - Export results to JSON/CSV and log deletion actions.

⚙️ **Configurable Settings** - Load and save settings using a JSON config file.

🔊 **Verbose & Quiet Modes** - Adjust output verbosity for better control.

---

## 🚀 Usage

```
dirpurge [OPTIONS] <path>
```

### 📂 Arguments

- `<path>` (**Required**) - Base directory to search.

### ⚙️ Options

| Option | Alias | Description |
|--------|-------|-------------|
| `-t, --target <target>` | | 🔎 Specify directory names to search for (multiple allowed). Default: `venv .venv node_modules target bin build` |
| `-e, --exclude <exclude>` | | 🚫 Exclude specific directories from search |
| `--depth <depth>` | | 📏 Set maximum search depth (0 = unlimited) |
| `--min-size <min-size>` | | 📦 Minimum directory size in MB to include |
| `--min-age <min-age>` | | 📅 Minimum age in days to include |
| `--follow-symlinks` | | 🔗 Follow symbolic links |
| `--delete` | | ❌ Perform deletion |
| `-y, --yes` | | ✅ Skip confirmation prompts |
| `-d, --dry-run` | | 🌵 Simulate operations without making changes |
| `--use-trash` | | 🗑 Move to trash instead of permanent deletion |
| `-b, --backup` | | 💾 Create backups before deletion |
| `-a, --archive` | | 📦 Create zip archives before deletion |
| `--backup-dir <DIR>` | | 📂 Specify backup/archive directory (default: `./backups`) |
| `-i, --interactive` | | 🖱 Select directories to delete interactively |
| `--confirm-phrase <confirm-phrase>` | | 🔐 Custom confirmation phrase for deletion (default: `DELETE`) |
| `--json <FILE>` | | 📄 Export results to JSON file |
| `--csv <FILE>` | | 📊 Export results to CSV file |
| `--log <FILE>` | | 📝 Write log to file |
| `-c, --config <FILE>` | | ⚙️ Load configuration from a JSON file |
| `--save-config <FILE>` | | 💾 Save current settings to a config file |
| `-v, --verbose` | | 🔊 Enable verbose output |
| `-q, --quiet` | | 🔈 Suppress non-essential output |
| `-h, --help` | | 📖 Show help information |
| `-V, --version` | | 🔢 Display version |

---

## 💡 Best Practices

- **Always run with `--dry-run` first** 🏜 to verify what will be deleted.
- **Use `--backup` or `--archive`** 💾 before permanent deletions.
- **Enable `--interactive` mode** 🖱 to manually confirm deletions.
- **Log everything** 📝 using `--log` for audit and troubleshooting.

---

## 📌 Examples

```sh
# Basic cleanup in a project directory
$ dirpurge ./project

# Remove only 'node_modules' directories
$ dirpurge ./src -t node_modules --delete

# Load settings from a configuration file
$ dirpurge . --config settings.json

# Interactive mode with safe deletion (move to trash)
$ dirpurge . -i --use-trash
```

---

## 🛠 Installation

> **Installation instructions will be added here.**

---

## 🏗 Configuration

### 🔧 Using a JSON Configuration File

Instead of passing multiple options in the command line, you can use a JSON config file:

```json
{
  "target": ["node_modules", "build"],
  "exclude": ["dist", "backup"],
  "min_size": 50,
  "min_age": 30,
  "delete": true,
  "use_trash": false,
  "backup": true,
  "backup_dir": "./backups",
  "log": "purge.log"
}
```

To use this configuration:
```sh
dirpurge ./projects --config settings.json
```

To save the current settings:
```sh
dirpurge ./projects --save-config settings.json
```

---

## 🏗 Roadmap

🔹 Improve multi-threaded performance for large-scale cleanup.

🔹 Add a GUI mode for users who prefer a visual interface.

🔹 Implement advanced analytics for better decision-making.

🔹 Provide more granular filtering (e.g., regex-based exclusions).

---

## 📜 License

This project is licensed under the **MIT License**.

---

## 🤝 Contributing

We welcome contributions! Please submit issues and pull requests for feature suggestions, bug fixes, and improvements.

### Steps to Contribute:
1. Fork the repository
2. Create a new branch (`feature-xyz`)
3. Commit your changes
4. Push to your fork
5. Create a pull request

---

## 📢 Support & Feedback

For any questions, bug reports, or feature requests, please open an issue or contact us directly.

Happy purging! 🚀