/// Realistic Cisco IOS-XE Software Version 17.09.04a output for Catalyst 9300-48P.
pub const SHOW_VERSION_C9300_48P: &str = r#"Cisco IOS XE Software, Version 17.09.04a
Cisco IOS Software [Cupertino], Catalyst L3 Switch Software (CAT9K_IOSXE), Version 17.9.4a, RELEASE SOFTWARE (fc2)
Technical Support: http://www.cisco.com/techsupport
Copyright (c) 1986-2023 by Cisco Systems, Inc.
Compiled Fri 28-Jul-23 04:12 by mcpre

ROM: IOS-XE ROMMON
BOOTLDR: System Bootstrap, Version 17.9.1r, RELEASE SOFTWARE (P)

sw-idf1a-01 uptime is 45 days, 2 hours, 11 minutes
Uptime for this control processor is 45 days, 2 hours, 13 minutes
System returned to ROM by reload
System image file is "bootflash:packages.conf"
Last reload reason: LocalSoft

This product contains cryptographic features and conforms to United
States and local country laws governing import, possession, use and
export of encryption products.

cisco C9300-48P (X86) processor with 1813733K/6147K bytes of memory.
Processor board ID FOC2419S3KL
1 Virtual Ethernet interface
52 Gigabit Ethernet interfaces
4 Ten Gigabit Ethernet interfaces
2048K bytes of non-volatile configuration memory.
16777216K bytes of physical memory.
11161600K bytes of Bootflash at bootflash:.
Base Ethernet MAC Address          : f4:0f:1b:2c:11:01
Motherboard Assembly Number        : 73-18288-04
Motherboard Serial Number          : FOC2419S3KL
Model Revision Number              : A0
Motherboard Revision Number        : B0
Model Number                       : C9300-48P
System Serial Number               : FOC2419S3KL

Configuration register is 0x102
"#;

pub const SHOW_VERSION_C9300_24P: &str = r#"Cisco IOS XE Software, Version 17.09.04a
Cisco IOS Software [Cupertino], Catalyst L3 Switch Software (CAT9K_IOSXE), Version 17.9.4a, RELEASE SOFTWARE (fc2)
Copyright (c) 1986-2023 by Cisco Systems, Inc.

sw-idf3c-01 uptime is 30 days, 8 hours, 12 minutes
System returned to ROM by reload
System image file is "bootflash:packages.conf"

cisco C9300-24P (X86) processor with 1813733K/6147K bytes of memory.
Processor board ID FOC2510A88B
28 Gigabit Ethernet interfaces
Base Ethernet MAC Address          : f4:0f:1b:2c:30:01
Model Number                       : C9300-24P
System Serial Number               : FOC2510A88B

Configuration register is 0x102
"#;

pub const SHOW_INTERFACES_STATUS_C9300_48P: &str = r#"Port      Name               Status       Vlan       Duplex  Speed Type
Gi1/0/1   Desk-E01           connected    10         a-full  a-1000 10/100/1000BaseTX
Gi1/0/2   Unused             notconnect   1            auto    auto 10/100/1000BaseTX
Gi1/0/4   Polycom VVX-450    connected    20         a-full  a-1000 10/100/1000BaseTX
Gi1/0/6   AP-06 Catalyst 91  connected    trunk      a-full  a-1000 10/100/1000BaseTX
Gi1/0/14  Rogue Switch BPDU  err-disabled 10           auto    auto 10/100/1000BaseTX
Gi1/0/47  Uplink to Core 01  connected    trunk      a-full   a-10G SFP-10GBase-SR
Gi1/0/48  Uplink to Core 02  connected    trunk      a-full   a-10G SFP-10GBase-SR
Te1/0/1                      notconnect   1            auto    auto SFP-10GBase-SR
"#;

pub const SHOW_INTERFACES_DETAIL_C9300: &str = r#"GigabitEthernet1/0/1 is up, line protocol is up (connected)
  Hardware is Gigabit Ethernet, address is f40f.1b2c.1101 (bia f40f.1b2c.1101)
  Description: Workstation Desk-E01
  MTU 1500 bytes, BW 1000000 Kbit/sec, DLY 10 usec,
     reliability 255/255, txload 1/255, rxload 1/255
  Encapsulation ARPA, loopback not set
  Keepalive set (10 sec)
  Full-duplex, 1000Mb/s, media type is 10/100/1000BaseTX
  input flow-control is off, output flow-control is unsupported
  ARP type: ARPA, ARP Timeout 04:00:00
  Last input 00:00:01, output 00:00:02, output hang never
  Last clearing of "show interface" counters never
  Input queue: 0/2000/0/0 (size/max/drops/flushes); Total output drops: 0
  Queueing strategy: fifo
  Output queue: 0/40 (size/max)
  5 minute input rate 12000 bits/sec, 15 packets/sec
  5 minute output rate 28000 bits/sec, 22 packets/sec
     14500000 packets input, 14500000000 bytes, 0 no buffer
     Received 120 broadcasts (0 multicasts)
     0 runts, 0 giants, 0 throttles
     0 input errors, 0 CRC, 0 frame, 0 overrun, 0 ignored
     0 watchdog, 0 multicast, 0 pause input
     0 input packets with dribble condition detected
     9800000 packets output, 8300000000 bytes, 0 underruns
     0 output errors, 0 collisions, 1 interface resets
     0 unknown protocol drops
     0 babbles, 0 late collision, 0 deferred
     0 lost carrier, 0 no carrier, 0 pause output
     0 output buffer failures, 0 output buffers swapped out
"#;

pub const SHOW_ENVIRONMENT_ALL_C9300: &str = r#"Sensor List: Environmental Monitor
 Sensor         Location        State           Reading
 Inlet          Switch 1        Normal          32 Celsius
 Hotspot        Switch 1        Normal          42 Celsius
 ASIC           Switch 1        Normal          48 Celsius

Power Supply:
 Switch#  PID                Serial No.     Status           Total  Sys Pwr  PoE Pwr  Watts
 1A       PWR-C1-715WAC-P    LIT24100XYZ    OK               715 W  165 W    550 W    715
 1B       PWR-C1-715WAC-P    LIT24100ABC    OK               715 W  165 W    550 W    715

Fan:
 Switch#  Fan Tray  Fan  Speed   State
 1        1         1    6500    OK
 1        1         2    6450    OK
 1        1         3    6520    OK
"#;

pub const SHOW_POWER_INLINE_C9300: &str = r#"Module   Available     Used     Remaining
          (Watts)     (Watts)    (Watts)
------   ---------   --------   ---------
1           740.0      142.5       597.5
Interface Admin  Oper       Power   Device              Class Max
                            (Watts)
--------- ------ ---------- ------- ------------------- ----- ----
Gi1/0/4   auto   on           7.4   IP Phone Polycom    2     15.4
Gi1/0/6   auto   on          25.5   AIR-AP9130AX        4     30.0
Gi1/0/8   auto   off          0.0   n/a                 n/a   30.0
Gi1/0/12  auto   on          15.4   CCTV Axis Cam       3     15.4
"#;

pub const SHOW_PROCESSES_CPU_C9300: &str = r#"CPU utilization for five seconds: 22%/4%; one minute: 20%; five minutes: 18%
 PID Runtime(ms)     Invoked      uSecs   5Sec   1Min   5Min TTY Process
 123     1204000     8400000        143  4.50%  4.20%  4.10%   0 IOS-XE Host
  45      450000     2100000        214  2.10%  2.00%  1.90%   0 SNMP engine
"#;

pub const SHOW_PROCESSES_MEMORY_C9300: &str = r#"Processor Pool Total: 16777216000 Used:  9730785280 Free:  7046430720
reserve Pool Total:            0 Used:           0 Free:           0
"#;

pub const SHOW_SWITCH_STACK_C9300: &str = r#"Switch/Stack Mac Address : f40f.1b2c.1101 - Local Mac Address
Mac persistency wait time: Indefinite
                                             H/W   Current
Switch#   Role    Mac Address     Priority Version  State
------------------------------------------------------------
*1       Active   f40f.1b2c.1101     15     V02     Ready
 2       Standby  f40f.1b2c.1102     14     V02     Ready
"#;
