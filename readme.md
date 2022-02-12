# GlRepo

A multi GIT project fetch tool inspired by google repo tool.

The manifest is YAML instead of XML:

```yaml
default_reference: main
projects_dir: src
projects:
  batchecker:
    fetch_url: git@git.gitlab.com/mike7b4/batchecker
    # Not needed if default_reference is specified above.
    reference: main
    # stored locally under src/batchecker
    path: batchecker
    # default is true so this field is not needed
    auto_sync: true
  stm32newboard-rs:
    fetch_url: git@git.gitlab.com/mike7b4/batchecker
    # stored locally under src/stm32newboard-rs since path is not specified
  linux:
    fetch_url: git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
    # If autosync is set to false the repo is not fetched when sync is run.
    # if not explicit specified with as *glrepo sync linux*
    auto_sync: false
```

# Features

 - [x] *sync* (optional [project] list)
 - [x] *list* project local *--path|--fetch-url|--reference|*
 - [x] run a shell command *for-each* project.
 - [x] Show *changed* projects
 - [x] *create* project

 # known issues

