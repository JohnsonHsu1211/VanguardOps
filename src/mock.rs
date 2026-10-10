use crate::model::*;

fn make_ports(prefix: &str, count: usize, switch_profile: &str) -> Vec<Interface> {
    let mut ports = Vec::with_capacity(count);

    for i in 1..=count {
        let name = format!("{prefix}{i}");
        let (
            admin,
            oper,
            mode,
            vlan,
            vlan_name,
            speed,
            poe_on,
            poe_w,
            desc,
            rx,
            tx,
            err_rx,
            err_tx,
            mac,
        ) = match (switch_profile, i) {
            // High-traffic uplink ports
            (_, 47 | 48) => (
                AdminStatus::Up,
                OperStatus::Up,
                PortMode::Trunk,
                1,
                "TRUNK_ALL".to_string(),
                "10 Gbps".to_string(),
                false,
                0.0,
                format!("Uplink to Core Trunk LACP [{i}]"),
                145_890_200_000,
                198_420_100_000,
                0,
                0,
                Some(format!("52:54:00:1a:2b:{i:02x}")),
            ),
            // BPDU Guard ErrDisabled port on switch 1A
            ("idf1a_01", 14) => (
                AdminStatus::Up,
                OperStatus::ErrDisabled,
                PortMode::Access,
                10,
                "CORP_DATA".to_string(),
                "1000 Mbps".to_string(),
                false,
                0.0,
                "Rogue Switch detected (BPDU-Guard Err-Disable)".to_string(),
                1_240_000,
                3_400_000,
                142,
                0,
                Some("00:11:22:33:44:55".to_string()),
            ),
            // Wireless AP ports with PoE+
            (_, i) if i % 6 == 0 => (
                AdminStatus::Up,
                OperStatus::Up,
                PortMode::Trunk,
                30,
                "CORP_WIFI".to_string(),
                "1000 Mbps".to_string(),
                true,
                25.5,
                format!("AP-{i:02} Cisco Catalyst 9130AX"),
                45_210_000_000,
                62_340_000_000,
                0,
                0,
                Some(format!("70:69:79:aa:bb:{i:02x}")),
            ),
            // VoIP Phones with PoE
            (_, i) if i % 4 == 0 => (
                AdminStatus::Up,
                OperStatus::Up,
                PortMode::Access,
                20,
                "VOIP_VOICE".to_string(),
                "1000 Mbps".to_string(),
                true,
                7.4,
                format!("IP Phone Polycom VVX-450 ext {i:03}"),
                2_100_000_000,
                1_800_000_000,
                0,
                0,
                Some(format!("00:04:f2:12:34:{i:02x}")),
            ),
            // Security cameras
            ("iot", i) if i <= 16 => (
                AdminStatus::Up,
                OperStatus::Up,
                PortMode::Access,
                50,
                "SEC_CAMERAS".to_string(),
                "1000 Mbps".to_string(),
                true,
                12.8,
                format!("Axis P3245-V CCTV Cam #{i:02}"),
                18_900_000_000,
                2_400_000_000,
                1,
                0,
                Some(format!("ac:cc:8e:44:55:{i:02x}")),
            ),
            // Regular workstation access ports (active)
            (_, i) if i % 2 == 1 && i <= 36 => (
                AdminStatus::Up,
                OperStatus::Up,
                PortMode::Access,
                10,
                "CORP_DATA".to_string(),
                "1000 Mbps".to_string(),
                false,
                0.0,
                format!("Workstation Desk-E{i:02}"),
                14_500_000_000,
                8_300_000_000,
                0,
                0,
                Some(format!("b8:27:eb:88:99:{i:02x}")),
            ),
            // Inactive ports (down)
            _ => (
                AdminStatus::Down,
                OperStatus::Down,
                PortMode::Access,
                1,
                "DEFAULT".to_string(),
                "Auto".to_string(),
                false,
                0.0,
                "Unused - Patch Panel Reserve".to_string(),
                0,
                0,
                0,
                0,
                None,
            ),
        };

        ports.push(Interface {
            name,
            description: desc,
            admin_status: admin,
            oper_status: oper,
            mode,
            vlan,
            vlan_name,
            speed,
            duplex: "Full".to_string(),
            poe_enabled: poe_on,
            poe_watts: poe_w,
            rx_bytes: rx,
            tx_bytes: tx,
            errors_rx: err_rx,
            errors_tx: err_tx,
            mtu: 1500,
            mac_learned: mac,
        });
    }

    ports
}

pub fn generate_mock_infrastructure() -> NetworkInfrastructure {
    // 1. MDF-MAIN (Data Center Core)
    let core_sw_01 = Switch {
        id: "sw-mdf-core01".to_string(),
        hostname: "core-sw-01.hq.net".to_string(),
        ip: "10.0.0.1".to_string(),
        mac: "00:1c:73:a1:00:01".to_string(),
        vendor: "Arista".to_string(),
        model: "7050SX3-48YC8".to_string(),
        serial: "JPE21045982".to_string(),
        os_version: "EOS 4.28.2F".to_string(),
        uptime: "184d 14h 22m".to_string(),
        cpu_usage: 14,
        mem_usage: 32,
        temp_celsius: 38,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Online,
        rack_unit: "U40-U41".to_string(),
        poe_budget_watts: 0.0,
        interfaces: make_ports("Ethernet", 48, "core"),
    };

    let core_sw_02 = Switch {
        id: "sw-mdf-core02".to_string(),
        hostname: "core-sw-02.hq.net".to_string(),
        ip: "10.0.0.2".to_string(),
        mac: "00:1c:73:a1:00:02".to_string(),
        vendor: "Arista".to_string(),
        model: "7050SX3-48YC8".to_string(),
        serial: "JPE21045983".to_string(),
        os_version: "EOS 4.28.2F".to_string(),
        uptime: "184d 14h 19m".to_string(),
        cpu_usage: 12,
        mem_usage: 31,
        temp_celsius: 37,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Online,
        rack_unit: "U38-U39".to_string(),
        poe_budget_watts: 0.0,
        interfaces: make_ports("Ethernet", 48, "core"),
    };

    let leaf_sw_01 = Switch {
        id: "sw-mdf-leaf01".to_string(),
        hostname: "leaf-sw-01.hq.net".to_string(),
        ip: "10.0.0.10".to_string(),
        mac: "00:1c:73:b2:00:10".to_string(),
        vendor: "Arista".to_string(),
        model: "7050SX3-48YC8".to_string(),
        serial: "JPE22019941".to_string(),
        os_version: "EOS 4.28.2F".to_string(),
        uptime: "92d 06h 45m".to_string(),
        cpu_usage: 18,
        mem_usage: 41,
        temp_celsius: 41,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Online,
        rack_unit: "U24-U25".to_string(),
        poe_budget_watts: 0.0,
        interfaces: make_ports("Ethernet", 48, "leaf"),
    };

    let rack_dc_01 = Rack {
        id: "rack-dc-01".to_string(),
        name: "Rack DC-01 (Spine)".to_string(),
        ru_capacity: 45,
        switches: vec![core_sw_01, core_sw_02],
    };

    let rack_dc_02 = Rack {
        id: "rack-dc-02".to_string(),
        name: "Rack DC-02 (Leaf)".to_string(),
        ru_capacity: 45,
        switches: vec![leaf_sw_01],
    };

    let mdf_closet = IdfCloset {
        id: "closet-mdf".to_string(),
        name: "MDF-MAIN (Data Center)".to_string(),
        building: "HQ Building 1".to_string(),
        floor: "Basement 1".to_string(),
        ambient_temp_celsius: 20.5,
        ups_status: "Online / Battery 100% (APC Symmetra LX)".to_string(),
        racks: vec![rack_dc_01, rack_dc_02],
    };

    // 2. IDF-1A (Floor 1 East)
    let idf1a_sw_01 = Switch {
        id: "sw-idf1a-01".to_string(),
        hostname: "sw-idf1a-01.hq.net".to_string(),
        ip: "10.1.10.11".to_string(),
        mac: "f4:0f:1b:2c:11:01".to_string(),
        vendor: "Cisco".to_string(),
        model: "Catalyst 9300-48P".to_string(),
        serial: "FOC2419S3KL".to_string(),
        os_version: "IOS-XE 17.09.04a".to_string(),
        uptime: "45d 02h 11m".to_string(),
        cpu_usage: 22,
        mem_usage: 58,
        temp_celsius: 42,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Master,
        status: DeviceStatus::Degraded, // BPDU error on port 14
        rack_unit: "U22".to_string(),
        poe_budget_watts: 740.0,
        interfaces: make_ports("Gi1/0/", 48, "idf1a_01"),
    };

    let idf1a_sw_02 = Switch {
        id: "sw-idf1a-02".to_string(),
        hostname: "sw-idf1a-02.hq.net".to_string(),
        ip: "10.1.10.12".to_string(),
        mac: "f4:0f:1b:2c:11:02".to_string(),
        vendor: "Cisco".to_string(),
        model: "Catalyst 9300-48P".to_string(),
        serial: "FOC2419S3KM".to_string(),
        os_version: "IOS-XE 17.09.04a".to_string(),
        uptime: "45d 02h 11m".to_string(),
        cpu_usage: 19,
        mem_usage: 54,
        temp_celsius: 40,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standby,
        status: DeviceStatus::Online,
        rack_unit: "U21".to_string(),
        poe_budget_watts: 740.0,
        interfaces: make_ports("Gi2/0/", 48, "idf1a_02"),
    };

    let idf1a_sw_iot = Switch {
        id: "sw-idf1a-iot-01".to_string(),
        hostname: "sw-idf1a-iot-01.hq.net".to_string(),
        ip: "10.1.50.21".to_string(),
        mac: "98:f2:b3:11:44:aa".to_string(),
        vendor: "Aruba".to_string(),
        model: "CX 6200F 48G".to_string(),
        serial: "CN89KS0012".to_string(),
        os_version: "AOS-CX 10.10.1020".to_string(),
        uptime: "112d 18h 03m".to_string(),
        cpu_usage: 15,
        mem_usage: 47,
        temp_celsius: 39,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::NotPresent,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Online,
        rack_unit: "U18".to_string(),
        poe_budget_watts: 370.0,
        interfaces: make_ports("1/1/", 48, "iot"),
    };

    let rack_1a_01 = Rack {
        id: "rack-1a-01".to_string(),
        name: "Rack 1A-UserAccess".to_string(),
        ru_capacity: 42,
        switches: vec![idf1a_sw_01, idf1a_sw_02],
    };

    let rack_1a_02 = Rack {
        id: "rack-1a-02".to_string(),
        name: "Rack 1A-BuildingIoT".to_string(),
        ru_capacity: 42,
        switches: vec![idf1a_sw_iot],
    };

    let idf1a_closet = IdfCloset {
        id: "closet-idf1a".to_string(),
        name: "IDF-1A (Floor 1 East)".to_string(),
        building: "HQ Building 1".to_string(),
        floor: "Floor 1".to_string(),
        ambient_temp_celsius: 22.8,
        ups_status: "Online / Battery 98% (APC Smart-UPS 3000)".to_string(),
        racks: vec![rack_1a_01, rack_1a_02],
    };

    // 3. IDF-2B (Floor 2 North)
    let idf2b_sw_01 = Switch {
        id: "sw-idf2b-01".to_string(),
        hostname: "sw-idf2b-01.hq.net".to_string(),
        ip: "10.1.20.11".to_string(),
        mac: "84:b5:9c:3d:20:01".to_string(),
        vendor: "Juniper".to_string(),
        model: "EX3400-48P".to_string(),
        serial: "PE3119042291".to_string(),
        os_version: "Junos 21.4R3-S4".to_string(),
        uptime: "78d 11h 50m".to_string(),
        cpu_usage: 28,
        mem_usage: 62,
        temp_celsius: 44,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Online,
        rack_unit: "U20".to_string(),
        poe_budget_watts: 740.0,
        interfaces: make_ports("ge-0/0/", 48, "idf2b_01"),
    };

    let idf2b_sw_02 = Switch {
        id: "sw-idf2b-02".to_string(),
        hostname: "sw-idf2b-02.hq.net".to_string(),
        ip: "10.1.20.12".to_string(),
        mac: "84:b5:9c:3d:20:02".to_string(),
        vendor: "Juniper".to_string(),
        model: "EX3400-48P".to_string(),
        serial: "PE3119042292".to_string(),
        os_version: "Junos 21.4R3-S4".to_string(),
        uptime: "78d 11h 48m".to_string(),
        cpu_usage: 30,
        mem_usage: 63,
        temp_celsius: 46,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::Fault, // PSU fault!
        fan_status: FanStatus::Warning,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Degraded,
        rack_unit: "U19".to_string(),
        poe_budget_watts: 740.0,
        interfaces: make_ports("ge-1/0/", 48, "idf2b_02"),
    };

    let rack_2b_01 = Rack {
        id: "rack-2b-01".to_string(),
        name: "Rack 2B-Main".to_string(),
        ru_capacity: 42,
        switches: vec![idf2b_sw_01, idf2b_sw_02],
    };

    let idf2b_closet = IdfCloset {
        id: "closet-idf2b".to_string(),
        name: "IDF-2B (Floor 2 North)".to_string(),
        building: "HQ Building 1".to_string(),
        floor: "Floor 2".to_string(),
        ambient_temp_celsius: 25.1,
        ups_status: "Online / Battery 95% (Eaton 9PX)".to_string(),
        racks: vec![rack_2b_01],
    };

    // 4. IDF-3C (Floor 3 Exec)
    let idf3c_sw_01 = Switch {
        id: "sw-idf3c-01".to_string(),
        hostname: "sw-idf3c-01.hq.net".to_string(),
        ip: "10.1.30.11".to_string(),
        mac: "f4:0f:1b:2c:30:01".to_string(),
        vendor: "Cisco".to_string(),
        model: "Catalyst 9300-24P".to_string(),
        serial: "FOC2510A88B".to_string(),
        os_version: "IOS-XE 17.09.04a".to_string(),
        uptime: "30d 08h 12m".to_string(),
        cpu_usage: 8,
        mem_usage: 39,
        temp_celsius: 36,
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::RedundantOk,
        fan_status: FanStatus::Normal,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Online,
        rack_unit: "U12".to_string(),
        poe_budget_watts: 445.0,
        interfaces: make_ports("Gi1/0/", 24, "idf3c_01"),
    };

    let rack_3c_01 = Rack {
        id: "rack-3c-01".to_string(),
        name: "Rack 3C-Exec".to_string(),
        ru_capacity: 24,
        switches: vec![idf3c_sw_01],
    };

    let idf3c_closet = IdfCloset {
        id: "closet-idf3c".to_string(),
        name: "IDF-3C (Floor 3 Exec)".to_string(),
        building: "HQ Building 1".to_string(),
        floor: "Floor 3".to_string(),
        ambient_temp_celsius: 21.0,
        ups_status: "Online / Battery 100% (APC Smart-UPS 1500)".to_string(),
        racks: vec![rack_3c_01],
    };

    // 5. IDF-WH (Warehouse Hub)
    let idfwh_sw_01 = Switch {
        id: "sw-idfwh-01".to_string(),
        hostname: "sw-idfwh-01.hq.net".to_string(),
        ip: "10.1.90.11".to_string(),
        mac: "f4:0f:1b:2c:90:01".to_string(),
        vendor: "Cisco".to_string(),
        model: "Catalyst 9300-24P".to_string(),
        serial: "FOC2341M11A".to_string(),
        os_version: "IOS-XE 17.09.04a".to_string(),
        uptime: "14d 03h 05m".to_string(),
        cpu_usage: 35,
        mem_usage: 51,
        temp_celsius: 54, // High temperature warning!
        psu_1: PowerSupplyStatus::Ok,
        psu_2: PowerSupplyStatus::NotPresent,
        fan_status: FanStatus::Warning,
        stack_role: StackRole::Standalone,
        status: DeviceStatus::Degraded,
        rack_unit: "U08".to_string(),
        poe_budget_watts: 445.0,
        interfaces: make_ports("Gi1/0/", 24, "idfwh_01"),
    };

    let rack_wh_01 = Rack {
        id: "rack-wh-01".to_string(),
        name: "Rack WH-Logistics".to_string(),
        ru_capacity: 24,
        switches: vec![idfwh_sw_01],
    };

    let idfwh_closet = IdfCloset {
        id: "closet-idfwh".to_string(),
        name: "IDF-WH (Logistics Warehouse)".to_string(),
        building: "Warehouse Logistics Bldg".to_string(),
        floor: "Ground Floor".to_string(),
        ambient_temp_celsius: 31.5,
        ups_status: "Online / Battery 92% (Tripp Lite SMART3000)".to_string(),
        racks: vec![rack_wh_01],
    };

    let alerts = vec![
        Alert {
            id: 1,
            timestamp: "2026-10-10 09:42:15".to_string(),
            severity: AlertSeverity::Critical,
            source: "sw-idf1a-01 (Gi1/0/14)".to_string(),
            message: "ERR_DISABLE: BPDU Guard error detected on port Gi1/0/14, port put into errdisabled state".to_string(),
        },
        Alert {
            id: 2,
            timestamp: "2026-10-10 09:30:00".to_string(),
            severity: AlertSeverity::Warning,
            source: "sw-idfwh-01".to_string(),
            message: "ENVMON_TEMP: Chassis internal temperature exceeded warning limit (54°C > threshold 50°C)".to_string(),
        },
        Alert {
            id: 3,
            timestamp: "2026-10-10 09:12:44".to_string(),
            severity: AlertSeverity::Warning,
            source: "sw-idf2b-02".to_string(),
            message: "ENVMON_PSU: Power Supply 2 fault detected, redundancy degraded, operating on single PSU".to_string(),
        },
        Alert {
            id: 4,
            timestamp: "2026-10-10 08:55:10".to_string(),
            severity: AlertSeverity::Info,
            source: "sw-idf1a-01".to_string(),
            message: "STACK_MGR: Stack member 2 (sw-idf1a-02) heartbeat acknowledged, election stable".to_string(),
        },
        Alert {
            id: 5,
            timestamp: "2026-10-10 08:15:33".to_string(),
            severity: AlertSeverity::Info,
            source: "core-sw-01".to_string(),
            message: "BGP_NEIGHBOR: Peer 10.0.0.2 ASN 65001 transitioned to ESTABLISHED state".to_string(),
        },
        Alert {
            id: 6,
            timestamp: "2026-10-10 07:02:19".to_string(),
            severity: AlertSeverity::Info,
            source: "leaf-sw-01".to_string(),
            message: "LLDP_NEIGHBOR: Discovered neighbor on Ethernet48 chassis ID 00:1c:73:a1:00:01".to_string(),
        },
    ];

    NetworkInfrastructure {
        closets: vec![
            mdf_closet,
            idf1a_closet,
            idf2b_closet,
            idf3c_closet,
            idfwh_closet,
        ],
        alerts,
    }
}
