pub fn pretty_print_ipv4(bytes: &[u8; 4]) -> String {
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

pub fn pretty_print_ipv6(bytes: &[u8; 16]) -> String {
    let mut groups = Vec::new();
    for i in 0..8 {
        let group = u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
        groups.push(group);
    }
    
    let mut best_start = 0;
    let mut best_len = 0;
    let mut current_start = 0;
    let mut current_len = 0;
    
    for (i, &group) in groups.iter().enumerate() {
        if group == 0 {
            if current_len == 0 {
                current_start = i;
            }
            current_len += 1;
        } else {
            if current_len > best_len {
                best_start = current_start;
                best_len = current_len;
            }
            current_len = 0;
        }
    }
    
    if current_len > best_len {
        best_start = current_start;
        best_len = current_len;
    }
    
    if best_len < 2 {
        best_len = 0;
    }
    
    let mut result = String::new();
    let mut i = 0;
    
    result.reserve(39);
    result.push('[');
    while i < 8 {
        if best_len > 0 && i == best_start {
            if i == 0 {
                result.push_str("::");
            } else {
                result.push(':');
            }
            i += best_len;
        } else {
            if i > 0 && !result.ends_with("::") {
                result.push(':');
            }
            result.push_str(&format!("{:x}", groups[i]));
            i += 1;
        }
    }
    result.push(']');
    
    result
}

pub fn pretty_print_mac(bytes: &[u8; 6]) -> String {
    format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5])
}