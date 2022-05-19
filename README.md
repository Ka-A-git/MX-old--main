# Trading Platform

Trading Platform

## Project structure

``` 

bench/                     Benchmarks
conf/                      This folder contains all configure files for server, platform, gateways and robots
+---gateway_config.toml    Gateway configure file
+---platform_config.toml   Platform configure file
+---robot_config.toml      Robot configure file
+---server_config.toml     Server configure file
data/
+---storage.db             It stores the state of the platform, gateways and robots when they stopped
+---events.log             This file stores all events that happened on platform
src/
+---api                    APIs for various stock exchanges
+---bin                    Contains server and cli main files
+---cli                    CLI utility for connecting to the Platform server and control its elements
+---config                 Config abstraction
+---context_manager        Context Manage
+---demo                   Generated Platform structure for demo
+---gateway                Gateway abstraction
+---logger                 Logging events on Platform
+---math                   Module for heavy math calculation
+---order_manager          Order manager
+---platform               Platform abstraction
+---robot                  Robot abstraction
    +---risk_control       Risk constrol abstraction
    +---strategy           Strategy abstraction
+---server                 HTTP Server
+---storage                Storage abstraction
test_files/                Files for testing Platform
```

## How to run

### Run Platform

``` 

cargo run --bin server
```

### Run CLI

``` 

cargo run --bin cli
```

### Run tests

``` 

cargo test
```

### Run benchmarks

``` 

cargo bench
```

### Run Demo

``` bash

cargo run --bin demo
```

## Trading Platform CLI

Command line interface for connecting to Trading Platform

### Commands

* `help` - Print all available commands
* `platform start` - Start the Platform
* `platform stop` - Stop the Platform
* `platform status` - Get status of the Platform
* `platform config <file_path>` - Set configuration for the Platform
* `robot start <robot_name>` - Start the Robot by name
* `robot stop <robot_name>` - Stop the Robot by name
* `robot status <robot_name>` - Get status of the Robot by name
* `robot info <robot_name>` - Get info of the Robot by name
* `robot config <robot_name> <file_path>` - Set configuration for the Robot
* `robot up` - Start all Robots
* `robot list` - Get all available Robots on the Platform
* `gateway start <gateway_name>` - Start the Gateway by name
* `gateway stop <gateway_name>` - Stop the Gateway by name
* `gateway status <gateway_name>` - Get status of the Gateway by name
* `gateway info <gateway_name>` - Get info of the Gateway by name
* `gateway config <gateway_name> <file_path>` - Set configuration for the Gateway
* `gateway up` - Start all Gateways
* `gateway list` - Get all available Gateways on the Platform
* `exit` - Disconnect from Trading Platform and quit

### Examples

#### Set configuration for Robot

 `robot config Robot1 conf/robot_config.toml`
