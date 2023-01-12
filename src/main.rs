use libp2p::{ core::upgrade
            , floodsub::{ Floodsub, FloodsubEvent, Topic                             }
            , futures::StreamExt
            , identity
            , mdns::{ Mdns, MdnsEvent                                                }
            , mplex
            , noise::{ Keypair, NoiseConfig, X25519Spec                              }
            , swarm::{ NetworkBehaviourEventProcess, Swarm, SwarmBuilder, SwarmEvent }
            , tcp::TokioTcpConfig
            , Multiaddr 
            , NetworkBehaviour
            , PeerId
            , Transport
}; //use libp2p::{ core::upgrade

use once_cell::sync::Lazy            ;
use serde::{ Deserialize, Serialize };
use sha2::{ Digest, Sha256          };
use std::{ thread, time             };
use tokio::{ io::AsyncBufReadExt    };

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Block { incoming: bool
             , previous: String
             , text    : String
             }
#[derive(Clone)]
struct Chat { chain       : Vec<Block>
            , interlocutor: String
            }
#[derive(Debug, Deserialize, Serialize)]
struct Letter { block   : Block
              , receiver: String
              }
#[derive(NetworkBehaviour)]
struct Behaviour { #[behaviour(ignore)] chats       : Vec<Chat>
                 ,                      floodsub    : Floodsub
                 , #[behaviour(ignore)] interlocutor: String
                 ,                      mdns        : Mdns    
                 , #[behaviour(ignore)] peer        : PeerId
                 }
enum Event { Address(Multiaddr)
           , Input(String)
           }
pub static DELAY: Lazy<u64> = Lazy::new(|| 400);

fn block(chain: &Vec<Block>, text: &String) -> Block {
 let mut previous = "                                                                ".to_string();

 if chain.len() > 0 {
  previous = hash(&chain.last().unwrap());

 }//if chain.len() > 0 {

 let block = Block { incoming: false, previous, text: text.clone() };

 block
}//fn block(chain: &Vec<Block>, text: &String) -> Block {

fn hash(block: &Block) -> String {
 let mut hasher = Sha256::new();

 hasher.update(serde_json::json!({"previous": block.previous, "text": block.text }).to_string().as_bytes());

 hex::encode(hasher.finalize().as_slice().to_owned())
}//fn hash(block: &Block) -> String {

fn print(chain: &Vec<Block>) {
 for link in chain {
  if link.incoming {
   println!("me: {:?}", link.text); thread::sleep(time::Duration::from_millis(*DELAY));

  } else {//if link.incoming {
   println!("my: {:?}", link.text); thread::sleep(time::Duration::from_millis(*DELAY));

  }//} else {//if link.incoming {
 }//for link in chain {
}//fn print(chain: &Vec<Block>) {

fn push(block: &Block, chain: &mut Vec<Block>) {
 if chain.len() > 0 {
  let last = chain.last().expect("there is at last block");

  if hash(last) == block.previous {
   chain.push(block.clone());

  } else {//if hash(last) == block.previous {
   println!("Error at message {:?}", block.text); thread::sleep(time::Duration::from_millis(*DELAY));

  }//} else {//if hash(last) == block.previous {

 } else {//if chain.len() > 0 {
  chain.push(block.clone());

 }//} else {//if chain.len() > 0 {
}//fn push(block: &Block, chain: &mut Vec<Block>) {

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
 fn inject_event(&mut self, event: FloodsubEvent) {
  match event {
   FloodsubEvent::Message(message) => {
    if let Ok(letter) = serde_json::from_slice::<Letter>(&message.data) {
     if letter.receiver.trim().is_empty() || letter.receiver == self.peer.to_string() {
      if let Some(index) = self.chats.iter().position(|x| x.interlocutor == message.source.to_string()) {
       push(&letter.block, &mut self.chats[index].chain);

      } else {//if let Some(index) = self.chats.iter().position(|x| x.interlocutor == message.source.to_string()) {
       let mut chat = Chat { chain: vec![], interlocutor: message.source.to_string() };

       push(&letter.block, &mut chat.chain);

       self.chats.push(chat);
      }//} else {//if let Some(index) = self.chats.iter().position(|x| x.interlocutor == message.source.to_string()) {

      if message.source.to_string() == self.interlocutor {
       println!("{:?}", &letter.block.text); thread::sleep(time::Duration::from_millis(*DELAY));

      } else {//if message.source.to_string() == self.interlocutor {
       println!("{:?} from {:?}", &letter.block.text, message.source); thread::sleep(time::Duration::from_millis(*DELAY));

      }//} else {//if message.source.to_string() == self.interlocutor {
     }//if letter.receiver.trim().is_empty() || letter.receiver == self.peer.to_string() {
    }//if let Ok(letter) = serde_json::from_slice::<Letter>(&message.data) {
   }//FloodsubEvent::Message(message) => {

   _ => ()
  }//match event {
 }//fn inject_event(&mut self, event: FloodsubEvent) {
}//impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
 fn inject_event(&mut self, event: MdnsEvent) {
  match event {
   MdnsEvent::Discovered(discovered) => { 
    for (peer, addr) in discovered {         
     self.floodsub.add_node_to_partial_view(peer);   

     println!("discovered: {:?}, {:?}", peer, addr); thread::sleep(time::Duration::from_millis(*DELAY)); 
    }//for (peer, addr) in discovered {         
   }//MdnsEvent::Discovered(discovered) => { 

   MdnsEvent::Expired(expired) => { 
    for (peer, addr) in expired {
     if !self.mdns.has_node(&peer) { 
      self.floodsub.remove_node_from_partial_view(&peer);

      println!("discovered: {:?}, {:?}", peer, addr); thread::sleep(time::Duration::from_millis(*DELAY)); 
     }//if !self.mdns.has_node(&peer) { 
    }//for (peer, addr) in expired {
   }//MdnsEvent::Expired(expired) => { 
  }//match event {
 }//fn inject_event(&mut self, event: MdnsEvent) {
}//impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {

#[tokio::main]
async fn main() {
 let identity: identity::Keypair = identity::Keypair::generate_ed25519();

 let peer: PeerId = PeerId::from(identity.public());

 let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&identity).expect("can create auth keys");

 let transp = TokioTcpConfig::new().upgrade(upgrade::Version::V1).authenticate(NoiseConfig::xx(auth_keys).into_authenticated()).multiplex(mplex::MplexConfig::new()).boxed();

 let topic: Topic = Topic::new("text");

 let mut behaviour = Behaviour { chats: vec![], floodsub: Floodsub::new(peer.clone()), interlocutor: "".to_string(), mdns: Mdns::new(Default::default()).await.expect("can create mdns"), peer };

 behaviour.floodsub.subscribe(topic.clone());

 let mut swarm = SwarmBuilder::new(transp, behaviour, peer.clone()).executor(Box::new(|fut| { tokio::spawn(fut); })).build();

 let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

 Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket")).expect("swarm can be started");

 println!("peer.to_string(): {:?}", peer.to_string()); thread::sleep(time::Duration::from_millis(*DELAY));

 loop { 
  let option = { 
   tokio::select! { event = swarm.select_next_some() => { match event { SwarmEvent::NewListenAddr { address, .. } => { Some(Event::Address(address)) } 
                                                                        _                                         =>   None                          
                                                                      } 
                                                        }
                  , line  = stdin.next_line()        => Some(Event::Input(line.expect("can get line").expect("can read line from stdin")))
                  }
  };//let option = { 

  if let Some(event) = option {
   match event {
    Event::Address(address) => {
     println!("Event::Address(address): {:?}", address); thread::sleep(time::Duration::from_millis(*DELAY));

    }//Event::Address(address) => {

    Event::Input(line) => {
     match &line[..] { "exit" => break
                     , "size" => println!("swarm.behaviour().mdns.discovered_nodes().len(): {:?}", swarm.behaviour().mdns.discovered_nodes().len())
                     , _      => if line.len() == 52 {
                                  if swarm.behaviour().mdns.discovered_nodes().find(|x| x.to_string() == line).is_some() {
                                   if let Some(chat) = swarm.behaviour().chats.iter().find(|x| x.interlocutor == line) {
                                    print(&chat.chain);

                                    swarm.behaviour_mut().interlocutor = line;

                                   } else {//if let Some(chat) = swarm.behaviour().chats.iter().find(|x| x.interlocutor == line) {
                                    let mut behaviour = swarm.behaviour_mut();

                                    behaviour.interlocutor = line.clone();

                                    behaviour.chats.push(Chat { chain: vec![], interlocutor: line });
                                   }//} else {//if let Some(chat) = swarm.behaviour().chats.iter().find(|x| x.interlocutor == line) {

                                  } else {//if swarm.behaviour().mdns.discovered_nodes().find(|x| x.to_string() == line).is_some() {
                                   let interlocutor = swarm.behaviour().interlocutor.clone();

                                   if let Some(index) = swarm.behaviour().chats.iter().position(|x| x.interlocutor == interlocutor) {
                                    let     behaviour: &mut Behaviour  =      swarm.behaviour_mut()       ;
                                    let     chain    : &mut Vec<Block> = &mut behaviour.chats[index].chain;
                                    let mut block    :      Block      =      block(&chain, &line)        ;

                                    push(&block, chain);

                                    block.incoming = true;

                                    let letter: Letter = Letter { block, receiver: interlocutor };

                                    behaviour.floodsub.publish(topic.clone(), serde_json::to_string(&letter).expect("can jsonify request").as_bytes());
                                   }//if let Some(index) = swarm.behaviour().chats.iter().position(|x| x.interlocutor == interlocutor) {
                                  }//} else {//if swarm.behaviour().mdns.discovered_nodes().find(|x| x.to_string() == line).is_some() {

                                 } else {//if line.len() == 52 { 
                                  let interlocutor = swarm.behaviour().interlocutor.clone();

                                  if let Some(index) = swarm.behaviour().chats.iter().position(|x| x.interlocutor == interlocutor) {
                                   let     behaviour: &mut Behaviour  =      swarm.behaviour_mut()       ;
                                   let     chain    : &mut Vec<Block> = &mut behaviour.chats[index].chain;
                                   let mut block    :      Block      =      block(&chain, &line)        ;

                                   push(&block, chain);

                                   block.incoming = true;

                                   let letter: Letter = Letter { block, receiver: interlocutor };

                                   behaviour.floodsub.publish(topic.clone(), serde_json::to_string(&letter).expect("can jsonify request").as_bytes());
                                  }//if let Some(index) = swarm.behaviour().chats.iter().position(|x| x.interlocutor == interlocutor) {
                                 }//} else {//if line.len() == 52 {
     }//match &line[..] {
    }//Event::Input(line) => {
   }//match event {
  }//if let Some(event) = option {
 }//loop {
}//async fn main() {
