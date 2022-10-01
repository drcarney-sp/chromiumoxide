use std::{
  fmt::Debug,
  sync::{Arc, Mutex},
  time::Duration,
};

use futures::StreamExt;
use log::info;

use chromiumoxide::{
  browser::{Browser, BrowserConfig},
  cdp::{
    browser_protocol::{
      dom::{DescribeNodeParams, GetFrameOwnerParams},
      page::{CreateIsolatedWorldParams, FrameId},
      target::{GetTargetsParams, SessionId, TargetId},
    },
    CdpEvent,
  },
  handler::GlobalEventListener,
};
use chromiumoxide_cdp::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams, EvaluateParams};

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  std::env::set_var("RUST_LOG", "debug");
  pretty_env_logger::init();
  log::info!("here");
  // if true {
  //   return Ok(());
  // }

  let ws_url = "ws://127.0.0.1:9222/devtools/browser/45d39d62-2aa4-493f-8843-02e5c8dd0d2a";
  let (browser, mut handler) = Browser::connect(ws_url, None).await?;
  let targets = Arc::new(Targets::default());
  let listener = Box::new(GlobalEventListenerImpl {
    targets: targets.clone(),
  });
  handler.set_global_event_listener(listener);
  let handle = async_std::task::spawn(async move {
    loop {
      let _ = handler.next().await.unwrap();
    }
  });

  async_std::task::sleep(Duration::from_millis(2_500)).await;
  let src = "https://cdn.privacy-mgmt.com/index.html?message_id";
  loop {
    info!("waiting for keypess for",);
    let _ = std::io::stdin()
      .read_line(&mut String::new())
      .unwrap_or(0);
    let pages = browser.pages().await?;
    let captured = targets.get_frames();
    for page in pages {
      log::info!("page {:?}", page);
      let frames = page.frames().await?;
      for frame in frames {
        info!("frame {:?}", frame);
      }
      let page_targets = page
        .execute(GetTargetsParams::default())
        .await?;

      for target in &page_targets.target_infos {
        info!("target {:?}", target);

        if target.r#type == "iframe" && target.attached && target.url.starts_with(src) {
          let mut session_id = None;
          for captured_target in &captured {
            if captured_target.target_id == target.target_id {
              info!("got with session {:?}", captured_target);
              session_id = Some(
                captured_target
                  .session_id
                  .clone(),
              );
            }
          }
          // let frame_id = target.url
          let frame_id = &target.target_id;
          let frame_id = FrameId::new(frame_id.inner());
          log::warn!("trying to create isolated world for frame {:?}", frame_id);
          // browser.execute_with_session(cmd, )
          let res = browser
            .execute_with_session(CreateIsolatedWorldParams::new(frame_id.clone()), session_id.clone())
            .await;
          match res {
            Ok(result) => {
              log::info!("{:?}", result.result);
              let eval_params = EvaluateParams::builder()
                .expression("document.body")
                .context_id(
                  result
                    .result
                    .execution_context_id
                    .clone(),
                )
                .build()
                .unwrap();
              let eval_res = browser
                .execute_with_session(eval_params, session_id.clone())
                .await?;
              info!("eval res {:?} ", eval_res.result);
            }
            Err(e) => {
              log::error!("failed {:?}", e);
              // let req = GetFrameOwnerParams::new(frame_id);
              // let res = page.execute(req).await?;
              // let owner = res.result;
              // log::info!("got owner {:?}", owner);
              // let req = DescribeNodeParams::builder()
              //   .backend_node_id(owner.backend_node_id)
              //   .build();
              // let res = page.execute(req).await?;
              // let owner = res.result;
              // log::info!("got owner {:?}", owner);
            }
          }
        }
      }
    }
  }

  handle.await;
  Ok(())
}

#[derive(Clone, Debug)]
struct TargetData {
  target_id: TargetId,
  session_id: SessionId,
}

#[derive(Default)]
struct Targets {
  targets: Mutex<Vec<TargetData>>,
}

impl Targets {
  fn add_frame(&self, target: TargetData) {
    if let Ok(mut guard) = self.targets.lock() {
      guard.push(target);
    }
  }

  fn get_frames(&self) -> Vec<TargetData> {
    if let Ok(guard) = self.targets.lock() {
      guard.clone()
    } else {
      Vec::new()
    }
  }
}

struct GlobalEventListenerImpl {
  targets: Arc<Targets>,
}

impl Debug for GlobalEventListenerImpl {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("asdf"))
  }
}

impl GlobalEventListener for GlobalEventListenerImpl {
  fn process_event(&self, message: &chromiumoxide::cdp::CdpEventMessage) {
    match &message.params {
      // CdpEvent::NetworkResponseReceived(e) => self.process_network_response_received(e),
      // CdpEvent::NetworkRequestWillBeSent(e) => self.process_network_request_will_be_sent(e),
      CdpEvent::PageFrameAttached(e) => {
        log::info!("{:?}", e)
      }
      CdpEvent::PageFrameDetached(e) => {
        log::info!("{:?}", e);
      }
      CdpEvent::DebuggerPaused(e) => log::info!("{:?}", e),
      CdpEvent::RuntimeExecutionContextCreated(e) => log::info!("{:?}", e),
      CdpEvent::RuntimeExecutionContextDestroyed(e) => log::info!("{:?}", e),
      CdpEvent::TargetTargetCreated(e) => log::info!("{:?}", e),
      CdpEvent::TargetAttachedToTarget(e) => {
        log::info!("{:?}", e);
        let session_id = e.session_id.clone();
        let target_id = e
          .target_info
          .target_id
          .clone();
        let target = TargetData {
          session_id,
          target_id,
        };
        self.targets.add_frame(target);
      }
      // CdpEvent::Page(e) => log::info!("{:?}", e),
      CdpEvent::PageFrameNavigated(e) => log::info!("{:?}", e),
      CdpEvent::PageFrameStartedLoading(e) => log::info!("{:?}", e),
      CdpEvent::PageFrameStoppedLoading(e) => log::info!("{:?}", e),
      _ => (),
    }
  }
}
