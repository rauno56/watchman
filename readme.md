## Watchman

WIP Utility for keeping processes running. Built for ssh tunnels and kubectl port forwarding.

```
watchman # interactive toggle
watchman add COMMAND [--name <name>] # adds new command to watch
watchman show # shows all commands and statuses
watchman config # prints out location of the config file
watchman fix # restarts died or dissapeared processes
```

### Interactive use

Running `watchman` shows an interactive list with configured processes. Navigate using arrow keys and toggle process with `space`.

```
$ watchman
Pick processes you want to be running:
> [ ] dev RabbitMQ -> 5678
  [ ] prod RabbitMQ -> 5679
  [x] my background script
```

### Overview

Running `watchman show` will give an overview of all processes.

```
 ? dev RabbitMQ -> 5678 [kubectl port-forward -n dev svc/rabbitmq 5678:5672]
   prod RabbitMQ -> 5679 [kubectl port-forward -n prod svc/rabbitmq 5679:5672]
 ✔ my background script [sh /home/rauno/projects/my-project/script]
```

* `✔`: The process is running.
* `?`: The process has died.
* ` `: The process is not configured to run.

Processes that have died or dissapeared can be restarted in bulk with `watchman fix`.
