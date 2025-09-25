# Web server

Implementation of a Web server / client using Rust. 

## :pushpin: Functionalities
 - Http query parser ( Server ) 
 - Screen sharing ( Server / Client )




# :postbox: Http query parser
A simple HTTP query parser allowing the server to retrieve : 
* The Http request **start line**
* The Http request **headers**
* The Http request optional **body**


# :vhs: Screen Sharing

**Screen sharing functionality sent through UDP without RTP overhead.**

---

## 🔧 UDP Video Streaming Workflow - End to End

> **System Overview:** Real-time video streaming system using UDP protocol with subscription-based client management and NAL unit processing.

---

## 📊 Video Streaming Flow Diagram

```
                    SERVER SIDE
                    ───────────
    [Video Source] 
          │
          ▼
    ┌─────────────────────┐
    │  Global Buffer      │ ◄── Store frames
    │  [F1][F2][F3][F4]   │
    └─────────┬───────────┘
              │ Dequeue one element
              ▼
    ┌─────────────────────┐
    │   UDP Transmitter   │ ◄── Send to all clients
    │    Broadcasting     │
    └─────────┬───────────┘
              │
              ▼
       UDP Network Layer
    ═══════════════════════
              │
              ▼
                    CLIENT SIDE
                    ──────────
    ┌─────────────────────┐
    │  First Buffer       │ ◄── Receive unordered
    │  [F3][F1][F4][F2]   │     packets
    └─────────┬───────────┘
              │ When threshold reached
              ▼
    ┌─────────────────────┐
    │   Sorting Process   │ ◄── Reorder packets
    │      [F1→F2→F3→F4]  │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │  Sorted Buffer      │ ◄── Ordered frames
    │  [F1][F2][F3][F4]   │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │  Frame Decoder      │ ◄── Decode & display
    │    & Display        │
    └─────────────────────┘
              │
              ▼
         [Screen Output]
```


---

## 🛠️ **Technical Stack**

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Transport** | UDP Sockets | Low-latency data transmission |
| **Video Codec** | H.264/H.265 | Efficient video compression |
| **Packetization** | NAL Units | Network-friendly video segments |
| **Buffering** | Multi-stage Buffers | Packet reordering & loss recovery |



