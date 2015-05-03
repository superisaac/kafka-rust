
use super::protocol::*;
use super::connection::*;
use super::codecs::*;
use std::collections::HashMap;
use std::io::Cursor;
use std::io::Read;

const CLIENTID: &'static str = "kafka-rust";
const DEFAULT_TIMEOUT: i32 = 120; // seconds


#[derive(Default)]
#[derive(Debug)]
pub struct KafkaClient {
    pub clientid: String,
    pub timeout: i32,
    pub hosts: Vec<String>,
    pub correlation: i32,
    pub conns: HashMap<String, KafkaConnection>,
    pub topic_partitions: HashMap<String, Vec<i32>>,
    pub topic_broker: HashMap<String, String>
}

impl KafkaClient {
    pub fn new(hosts: &Vec<String>) -> KafkaClient {
        KafkaClient { hosts: hosts.to_vec(), clientid: CLIENTID.to_string(),
                      timeout: DEFAULT_TIMEOUT, ..KafkaClient::default()}
    }

    fn get_conn(& mut self, host: &String) -> KafkaConnection {
        match self.conns.get(host) {
            Some (conn) => return conn.clone(),
            None => {}
        }
        // TODO
        // Keeping this out here since get is causing ownership issues
        // Will refactor once I know better
        self.conns.insert(host.clone(), KafkaConnection::new(host, self.timeout));
        self.get_conn(host)
    }

    fn next_id(&mut self) -> i32{
        self.correlation = (self.correlation + 1) % (1i32 << 30);
        self.correlation
    }


    pub fn load_metadata(&mut self, topics: &Vec<String>) {
        let resp = self.get_metadata(topics);
        let brokers: HashMap<i32, BrokerMetadata> = HashMap::new()
        for broker in resp.brokers {
            brokers.insert(broker.nodeid, format!("{}:{}", broker.host, broker.port));
        }

        self.topic_brokers.clear();
        for topic in topics {
            self.topic_partitions[topic] = Vec::new();
            for partition in topic.partitions {
                match brokers.get(partition.leader) {
                    Some(& broker) => {
                        self.topic_partitions[topic].push(partition.id);
                        self.topic_broker.insert(
                            format!("{}-{}", topic.topic, partition.id, broker)
                            broker)
                    },
                    None => {}
                }

            }
        }
    }

    pub fn reset_metadata(&mut self) {
        self.topic_partitions.clear();
        self.topic_broker.clear();
    }

    fn get_metadata(&mut self, topics: &Vec<String>) -> MetadataResponse {
        for host in self.hosts.to_vec() {
            let correlation = self.next_id();
            let req = MetadataRequest::new(correlation, &self.clientid, topics.to_vec());
            let mut conn = self.get_conn(&host);
            let sent = self.send_request(&mut conn, req);
            if (sent) {
                return self.get_response::<MetadataResponse>(&mut conn);
            }
        }
        panic!("All Brokers failes to process request!");
    }

    pub fn fetch_offset(& self, topic: String) {

    }

    fn send_request<T: ToByte>(&self, conn: &mut KafkaConnection, request: T) -> bool{
        let mut buffer = vec!();
        request.encode(&mut buffer);

        let mut s = vec!();
        (buffer.len() as i32).encode(&mut s);
        for byte in buffer.iter() { s.push(*byte); }
        let bytes_to_send = s.len();

        match conn.send(&s) {
            Ok(num) => return num == bytes_to_send,
            Err(e) => return false
        }
    }

    fn get_response<T: FromByte>(&self, conn:&mut KafkaConnection) -> T::R{
        let mut v: Vec<u8> = vec!();
        conn.read(4, &mut v);
        let size = i32::decode_new(&mut Cursor::new(v));

        let mut resp: Vec<u8> = vec!();
        conn.read(size as u64, &mut resp);
        T::decode_new(&mut Cursor::new(resp))
    }

}
