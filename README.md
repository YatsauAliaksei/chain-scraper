## Product vision
- - - -
CQRS:
* Command -> Postgres/Mongo ?. Raw data
* Query -> Elastic Search. Available once ABI uploaded.


## Basic usage

- - - -

* load all none empty blocks with transactions. Historical + runtime
    * save to DB
* load ABI
    * save ABI to db as trackable contract
* once ABI loaded enable:
    * create an Elastic index with all blocks/trx related to contract
      
    * Reports:
        * all trx in time range. Kibana?
    * Operations:
        * TBD

## todo:

### Block

- - - -

- [x] fetch range of blocks
- [x] fetch range of blocks in async/batch way
- [ ] (Optional) support multiple RPC node for traversal

### Transaction

- - - -

- [ ] Parse transaction
    - [ ] contract invocation. Method call
        - [ ] from contract
        - [x] to contract
    - [ ] (Optional) contract creation
        - [ ] (Optional) decompile from ABI 'eth_getCode'. As sample https://github.com/eveem-org/panoramix
    - [ ] (Optional) ETH move
    - [ ] other trx?

### Contract

- - - -

- [x] support contract Json representation upload
- [x] parse Json to create domain object
- [x] lookup by contract
    - [x] find all trx related to contract

### Web

- - - - 

- [x] Web server
    - [x] upload contract ABI

### Runtime

- - - - 

- [x] handle new blocks at runtime

### DB

- - - -

- [x] Save blocks

### GUI

 - - - -

- [ ] Basic web GUI
    - [ ] consider [Flutter](https://flutter.dev/)
    - [x] Kibana + Elastic Search