# polygon-data-relay
`polygon-data-relay` is an application for streaming data from the [polygon](https://polygon.io/) market-data provider to [apache kafka](https://kafka.apache.org/) in a fault-tolerant way. Messages from polygon are type-checked and only passed on if they conform to the built-in schema.

## Aspirational
Currently, `polygon-data-relay` emits data to kafka in json. In the future, data will be transmuted into a more efficient binary wire protocol such as protobuf using [serde-transcode](https://serde.rs/transcode.html).
