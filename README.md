# WEBSMQ WebSocket Message Queue

## What is it?

WEBSMQ is a very easy to implement WebSocket based pub/sub message queue.

## Features

A WEBSMQ client can:

### Subscribe to a topic

Subsequently it will receive all messages published to this topic until it either unsubscribes or disconnects.

### Publish to a topic

All other clients subscribed to that topic will receive the message.

### Declare a last will

The last will consists of a topic and a message. When the client disconnects, the broker will publish the last will message to the provided topic.

## Topics

Topics are hierarchical and can consist of path segments (e.g. sensors/temps/living_room). They must be URL encoded and contain only ASCII characters. Topics can have a maximum length of 16383 bytes.

## Protocol

WEBSMQ uses a binary protocol. Every message is composed of a header, a topic and - depending on the message type - an optional payload.

### Header

The header consists of two bytes where the first two bits of the first byte define the type of the message and the other six and the second byte define the length of the topic in bytes.

#### Message Type

The following message types are available:

| Type        | Bits |
| :---------- | :--: |
| Publish     |  00  |
| Subscribe   |  01  |
| Unsubscribe |  10  |
| Last Will   |  11  |

The broker never changes the type of the message, i.e. messages that are published by a client will be delivered to other clients with the 'Publish' type header, i.e. start with 00, messages delivered to clients by the broker as a last will due to a client disconnect will have type 'Last Will' i.e. start with 11.

The broker must never send messages of type 'Subscribe' (01) or 'Unsubscribe' (10) to clients.

### Topic

The topic is a byte array containing an ASCII encoded String. The number of bytes that belong to the topic are defined in the header. Any bytes coming after the topic belong to the payload.

### Payload

The Payload is an arbitrary binary blob that can contain any kind of data, for example a string, a protobuf message or a file.

'Subscribe' and 'Unsubscribe' messages will typically not have a payload, if they do, the server should ignore it.
