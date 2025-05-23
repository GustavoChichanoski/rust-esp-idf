# Heltec V2 (LoRa Communication)

The Heltec V2 is a LoRa communication module that can be used to send and receive data over a LoRa network. It is a wireless module that can be used to send and receive data over a LoRa network.

## SX1276 (LoRa 1.8MHz)

```mermaid
flowchart TD
    A[Start] --> B[Set FifoAddrPtr to FifoRxBaseAddr]
    B --> C[Write configuration registers in Sleep, Standby, or FSRX mode]
    C --> D[Select RXSINGLE mode to initiate single packet reception]
    D --> E[Receiver waits for valid preamble]
    E --> F[Set gain of receive chain after valid preamble]
    F --> G[Check for valid header: ValidHeader interrupt in explicit mode]
    G --> H[Begin packet reception process]
    H --> I[RxDone interrupt is set after packet reception completes]
    I --> J[Radio returns to Standby mode to reduce power consumption]
    J --> K[Check PayloadCrcError register for packet payload integrity]
    K --> L{Is payload valid?}
    L -- Yes --> M[Read FIFO for received packet payload]
    L -- No --> N[Discard payload]
    M --> O{Need another packet?}
    O -- Yes --> P[Reset SPI pointer to FifoRxBaseAddr and reselect RXSINGLE mode]
    O -- No --> Q[End]
```

| DIO Pin | Value | Event                         |
|---------|-------|-------------------------------|
| DIO0    | 00    | RxDone  / TxDone              |
| DIO0    | 01    | CadDone                       |
| DIO0    | 10    | FhssChangeChannel             |
| DIO0    | 11    | CadDetected                   |

| DIO Pin | Value | Event                         |
|---------|-------|-------------------------------|
| DIO1    | 00    | RxTimeout                     |
| DIO1    | 01    | FhssChangeChannel             |
| DIO1    | 10    | CadDetected                   |
| DIO1    | 11    | Reserved                      |

| DIO Pin | Value | Event                         |
|---------|-------|-------------------------------|
| DIO2    | 00    | FhssChangeChannel             |
| DIO2    | 01    | Reserved                      |
| DIO2    | 10    | CadDetected                   |
| DIO2    | 11    | Reserved                      |

| DIO Pin | Value | Event                         |
|---------|-------|-------------------------------|
| DIO3    | 00    | CadDone                       |
| DIO3    | 01    | ValidHeader / PayloadCrcError |
| DIO3    | 10    | Reserved                      |
| DIO3    | 11    | Reserved                      |

| DIO Pin | Value | Event                         |
|---------|-------|-------------------------------|
| DIO4    | 00    | PLLLock                       |
| DIO4    | 01    | RxReady                       |
| DIO4    | 10    | TxReady                       |
| DIO4    | 11    | Reserved                      |

| DIO Pin | Value | Event                         |
|---------|-------|-------------------------------|
| DIO5    | 00    | ModeReady                     |
| DIO5    | 01    | ClkOut                        |
| DIO5    | 10    | Reserved                      |
| DIO5    | 11    | Reserved                      |
#   c i a d i e s e l - r u s t - e s p - i d f  
 