# Pre-merge QA fixes

Task1 lib.rs: add regression preserving each non-CLEAN single-file outcome on accepted cancellation. Run filtered Rust test and observe failure. Change mark_user_cancelled to revoke only CLEAN; retain positive/error evidence while summary remains cancelled/incomplete. Run Rust tests.
Task2 App.tsx/run.mjs: native regression with51 folder files, first mock response INFECTED then50 CLEAN, assert51 visible retained details and threat path. Observe failure on current binary. Remove UI last50 slice; render backend-bounded1000 in scrollable list, preserving truncation notice when backend cap reached. Run native suite.
Then commit source, push draft PR, independently rebuild/retest exact SHA in Qeen checkout; record QA evidence, pending native picker/EICAR/release checks separately. No merge until gates pass.
