# GlRepo

A multi GIT project fetch tool inspired by google repo tool.

The manifest is YAML instead of XML:

```
default_reference: main
projects_dir: src
projects:
  batchecker:
    fetch_url: git@git.gitlab.com/mike7b4/batchecker
    # Not needed if default_reference is specified above.
    reference: main
    # stored locally under src/batchecker
    path: batchecker
    # default is true so this one is not needed
    auto_sync: true
  stm32newboard-rs:
    fetch_url: git@git.gitlab.com/mike7b4/batchecker
    # stored locally under src/stm32newboard-rs sinve path is not specified
  linux:
    fetch_url: git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
    # If autosync is set to false the repo is not fetched when ryn sync
    auto_sync: false
```

# Features

 - [x] *sync*
 - [x] *list* project local *--path|--fetch-url|--reference|*
 - [x] run a shell command *for-each* project.
 - [x] Show *changed* projects
 - [x] *create* project

 # known issues

