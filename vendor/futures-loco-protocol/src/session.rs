use std::{
    fmt::{self, Debug, Display},
    io, mem,
    pin::Pin,
    task::{Context, Poll},
};

use flume::{r#async::RecvStream, Receiver, Sender};
use futures_core::{ready, Future, Stream};
use futures_io::{AsyncRead, AsyncWrite};
use loco_protocol::command::{Header, Method};
use nohash_hasher::IntMap;

use crate::{BoxedCommand, LocoClient};

#[derive(Debug, Clone)]
pub struct LocoSession {
    sender: Sender<Outbound>,
}

impl LocoSession {
    pub fn new<T>(client: LocoClient<T>) -> (Self, LocoSessionStream<T>) {
        let (sender, receiver) = flume::bounded(16);

        (Self { sender }, LocoSessionStream::new(receiver, client))
    }

    pub async fn request(&self, method: Method, data: Vec<u8>) -> Result<CommandRequest, Error> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send_async(Outbound::Request(Request {
                method,
                data,
                response_sender: sender,
            }))
            .await
            .map_err(|_| Error::SessionClosed)?;

        Ok(CommandRequest { inner: receiver })
    }

    /// Queue a response that preserves the peer's packet header.
    pub async fn respond(&self, header: Header, data: Vec<u8>) -> Result<(), Error> {
        self.sender
            .send_async(Outbound::Response(Response { header, data }))
            .await
            .map_err(|_| Error::SessionClosed)
    }
}

pin_project_lite::pin_project!(
    pub struct LocoSessionStream<T> {
        #[pin]
        request_stream: RecvStream<'static, Outbound>,

        response_map: IntMap<u32, oneshot::Sender<BoxedCommand>>,

        state: SessionState,

        #[pin]
        client: LocoClient<T>,
    }
);

impl<T> LocoSessionStream<T> {
    fn new(request_receiver: Receiver<Outbound>, client: LocoClient<T>) -> Self {
        Self {
            request_stream: request_receiver.into_stream(),
            response_map: IntMap::default(),

            state: SessionState::Pending,

            client,
        }
    }
}

impl<T: AsyncRead + AsyncWrite> Stream for LocoSessionStream<T> {
    type Item = io::Result<BoxedCommand>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match mem::replace(this.state, SessionState::Done) {
                SessionState::Pending => {
                    while let Poll::Ready(read) = this.client.as_mut().poll_read(cx) {
                        let read = read?;

                        if let Some(sender) = this.response_map.remove(&read.header.id) {
                            let _ = sender.send(read);
                        } else {
                            *this.state = SessionState::Pending;
                            return Poll::Ready(Some(Ok(read)));
                        }
                    }

                    let mut receiver_read = false;
                    while let Poll::Ready(Some(outbound)) =
                        this.request_stream.as_mut().poll_next(cx)
                    {
                        match outbound {
                            Outbound::Request(request) => {
                                let id = this.client.as_mut().write(request.method, &request.data);
                                this.response_map.insert(id, request.response_sender);
                            }
                            Outbound::Response(response) => {
                                this.client
                                    .as_mut()
                                    .write_command(response.header, &response.data);
                            }
                        }

                        if !receiver_read {
                            receiver_read = true;
                        }
                    }

                    if receiver_read {
                        *this.state = SessionState::Write;
                    } else {
                        *this.state = SessionState::Pending;
                        return Poll::Pending;
                    }
                }

                SessionState::Write => {
                    if this.client.as_mut().poll_flush(cx)?.is_ready() {
                        *this.state = SessionState::Pending;
                    } else {
                        *this.state = SessionState::Write;
                        return Poll::Pending;
                    };
                }

                SessionState::Done => return Poll::Ready(None),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SessionState {
    Pending,
    Write,
    Done,
}

#[derive(Debug)]
struct Request {
    method: Method,
    data: Vec<u8>,
    response_sender: oneshot::Sender<BoxedCommand>,
}

#[derive(Debug)]
struct Response {
    header: Header,
    data: Vec<u8>,
}

#[derive(Debug)]
enum Outbound {
    Request(Request),
    Response(Response),
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    pub struct CommandRequest {
        #[pin]
        inner: oneshot::Receiver<BoxedCommand>,
    }
}

impl Future for CommandRequest {
    type Output = Result<BoxedCommand, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let command = ready!(self
            .project()
            .inner
            .poll(cx)
            .map_err(|_| Error::SessionClosed))?;

        Poll::Ready(Ok(command))
    }
}

#[derive(Debug)]
pub enum Error {
    SessionClosed,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("session closed")
    }
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
    };

    use futures_core::Stream;
    use futures_io::{AsyncRead, AsyncWrite};
    use futures_lite::future;
    use loco_protocol::command::{client::LocoStream, Header, Method};

    use super::*;

    #[derive(Clone, Default)]
    struct PendingIo {
        written: Arc<Mutex<Vec<u8>>>,
    }

    impl AsyncRead for PendingIo {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Pending
        }
    }

    impl AsyncWrite for PendingIo {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.written.lock().unwrap().extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn response_reuses_explicit_header_without_waiting_for_a_reply() {
        let io = PendingIo::default();
        let written = io.written.clone();
        let (session, mut stream) = LocoSession::new(LocoClient::new(io));
        let header = Header {
            id: 73,
            status: 9,
            method: Method::new("MSG").unwrap(),
            data_type: 0,
        };

        future::block_on(session.respond(header.clone(), vec![1, 2, 3])).unwrap();
        future::block_on(future::poll_fn(|cx| {
            assert!(Pin::new(&mut stream).poll_next(cx).is_pending());
            Poll::Ready(())
        }));

        let bytes = written.lock().unwrap().clone();
        let mut decoder = LocoStream::new();
        decoder.read_buffer.extend(bytes);
        let command = decoder.read().unwrap();

        assert_eq!(command.header, header);
        assert_eq!(&*command.data, &[1, 2, 3]);
    }
}
