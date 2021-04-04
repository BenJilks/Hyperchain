use std::sync::mpsc::{channel, Sender, Receiver};

pub struct Broadcaster<T: Clone>
{
    senders: Vec<Sender<T>>,
}

impl<T: Clone> Broadcaster<T>
{

    pub fn new() -> Self
    {
        Self
        {
            senders: Vec::new(),
        }
    }

    pub fn make_receiver(&mut self) -> Receiver<T>
    {
        let (send, recv) = channel();
        self.senders.push(send);
        recv
    }

    pub fn broadcast(&mut self, t: T)
    {
        let mut senders_to_keep = Vec::<Sender<T>>::new();
        for sender in &self.senders 
        {
            if sender.send(t.clone()).is_ok() {
                senders_to_keep.push(sender.clone());
            }
        }

        self.senders = senders_to_keep;
    }

}
