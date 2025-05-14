use crate::Trace;

#[derive(Debug)]
/// A span, meaning a start trace and an end trace.
#[allow(dead_code)]
pub(crate) struct Span<'a> {
    /// The start trace where the span started. This contains most of the information
    pub(crate) start: &'a Trace,
    /// The end of the span given in trace format.
    pub(crate) end: &'a Trace,
}

fn queue_modify(trace: &Trace, start: &Trace, queue_size: &mut i32) {
    if trace.pid == start.pid && trace.cpu == start.cpu {
        *queue_size = match trace.trace_marker {
            crate::trace::TraceMarker::StartSync => *queue_size + 1,
            crate::trace::TraceMarker::EndSync => *queue_size - 1,
            crate::trace::TraceMarker::StartAsync
            | crate::trace::TraceMarker::EndAsync
            | crate::trace::TraceMarker::Dot => *queue_size,
        };
    }
}

/// Finds the end of the span that starts at `start_pos` in `traces`
fn find_end(start_pos: usize, traces: &[Trace]) -> Option<usize> {
    let mut queue_size = 1;
    let start = traces.get(start_pos).unwrap();
    let mut iter = traces.iter().skip(start_pos + 1).map(|trace| {
        queue_modify(trace, start, &mut queue_size);
        queue_size
    });

    iter.position(|queue_size| queue_size == 0)
        .map(|pos| start_pos + 1 + pos)
}

/// Returns all spans that match the function with fn_name. This is an exact match.
/// Because this is a relative simple algorithm, finding all all traces takes roughly O(n).
pub(crate) fn find_all_spans(fn_name: String, traces: &[Trace]) -> Vec<Span> {
    let mut spans = Vec::new();
    let start_traces = traces
        .iter()
        .enumerate()
        .filter(|(_index, t)| t.function == fn_name);

    for (start_position, start_trace) in start_traces {
        let end_position = find_end(start_position, traces).unwrap();
        let end = traces.get(end_position).unwrap();
        let s = Span {
            start: start_trace,
            end,
        };
        spans.push(s);
    }

    spans
}

#[cfg(test)]
mod tests {
    use crate::{
        Trace,
        trace::{TimeStamp, TraceMarker},
    };

    use super::*;

    fn new_trace(name: &str, pid: u64, seconds: u64, marker: TraceMarker) -> Trace {
        Trace {
            name: String::from(name),
            pid,
            cpu: 1,
            timestamp: TimeStamp { seconds, micro: 0 },
            trace_marker: marker,
            number: String::from("1"),
            shorthand: "f".to_string(),
            function: String::from(name),
        }
    }

    #[test]
    fn test_find_next_for_one() {
        let traces = vec![
            new_trace("Foo", 1, 1, TraceMarker::StartSync),
            new_trace("Foo2", 1, 2, TraceMarker::StartSync),
            new_trace("", 1, 3, TraceMarker::EndSync),
            new_trace("", 1, 4, TraceMarker::EndSync),
        ];

        let res = find_end(0, &traces);
        assert_eq!(res, Some(3));
        let end_trace = traces.get(res.unwrap());
        assert_eq!(end_trace.map(|t| t.timestamp.seconds), Some(4));
    }

    #[test]
    fn test_find_next_for_multiple() {
        let traces = vec![
            new_trace("Foo", 1, 1, TraceMarker::StartSync),
            new_trace("Foo2", 1, 2, TraceMarker::StartSync),
            new_trace("", 1, 3, TraceMarker::EndSync),
            new_trace("Foo2", 1, 4, TraceMarker::StartSync),
            new_trace("", 1, 5, TraceMarker::EndSync),
            new_trace("", 1, 6, TraceMarker::EndSync),
            new_trace("Foo2", 1, 7, TraceMarker::StartSync),
            new_trace("Foo2", 1, 8, TraceMarker::StartSync),
            new_trace("Foo2", 1, 9, TraceMarker::StartSync),
        ];

        let res = find_end(0, &traces);
        assert_eq!(res, Some(5));
        let end_trace = traces.get(res.unwrap());
        assert_eq!(end_trace.map(|t| t.timestamp.seconds), Some(6));
    }

    #[test]
    fn test_find_spans() {
        let traces = vec![
            new_trace("Foo", 1, 1, TraceMarker::StartSync), // Foo starts
            new_trace("Foo2", 1, 2, TraceMarker::StartSync),
            new_trace("", 1, 3, TraceMarker::EndSync),
            new_trace("Foo2", 1, 4, TraceMarker::StartSync),
            new_trace("", 1, 5, TraceMarker::EndSync),
            new_trace("", 1, 6, TraceMarker::EndSync), //Foo ends
            new_trace("Foo", 1, 7, TraceMarker::StartSync), // Foo starts
            new_trace("Foo2", 1, 8, TraceMarker::StartSync),
            new_trace("Foo2", 1, 9, TraceMarker::StartSync),
            new_trace("Foo2", 1, 10, TraceMarker::StartSync),
            new_trace("", 1, 11, TraceMarker::EndSync),
            new_trace("", 1, 12, TraceMarker::EndSync),
            new_trace("", 1, 13, TraceMarker::EndSync),
            new_trace("", 1, 14, TraceMarker::EndSync), // Foo ends
        ];

        let res = find_all_spans(String::from("Foo"), &traces);
        assert_eq!(res.len(), 2);
        let first_span = &res[0];
        let snd_span = &res[1];

        // First span
        assert_eq!(first_span.start.function, "Foo");
        assert_eq!(first_span.start.timestamp.seconds, 1);
        assert_eq!(first_span.end.timestamp.seconds, 6);

        // Second span
        assert_eq!(snd_span.start.function, "Foo");
        assert_eq!(snd_span.start.timestamp.seconds, 7);
        assert_eq!(snd_span.end.timestamp.seconds, 14);
    }

    #[test]
    fn test_find_spans_ignore_wrong_pid() {
        let traces = vec![
            new_trace("Foo", 1, 1, TraceMarker::StartSync), // Foo starts
            new_trace("Foo2", 1, 2, TraceMarker::StartSync),
            new_trace("", 1, 3, TraceMarker::EndSync),
            new_trace("Foo2", 1, 4, TraceMarker::StartSync),
            new_trace("", 1, 5, TraceMarker::EndSync),
            new_trace("", 2, 6, TraceMarker::EndSync), //Foo does notends
            new_trace("Foo2", 1, 8, TraceMarker::StartSync),
            new_trace("Foo2", 1, 9, TraceMarker::StartSync),
            new_trace("Foo2", 1, 10, TraceMarker::StartSync),
            new_trace("", 1, 11, TraceMarker::EndSync),
            new_trace("", 1, 12, TraceMarker::EndSync),
            new_trace("", 1, 13, TraceMarker::EndSync),
            new_trace("", 1, 14, TraceMarker::EndSync), // Foo ends
        ];

        let res = find_all_spans(String::from("Foo"), &traces);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].start.timestamp.seconds, 1);
        assert_eq!(res[0].end.timestamp.seconds, 14);
    }

    #[test]
    fn test_if_ignore_others_long() {
        let traces = vec![
            new_trace("Foo", 1, 1, TraceMarker::StartSync), // Foo starts
            new_trace("Foo2", 1, 2, TraceMarker::StartSync),
            new_trace("", 1, 3, TraceMarker::EndSync),
            new_trace("Foo2", 1, 4, TraceMarker::StartSync),
            new_trace("", 1, 5, TraceMarker::EndSync),
            new_trace("Foo2", 1, 6, TraceMarker::StartSync),
            new_trace("", 1, 7, TraceMarker::EndSync),
            new_trace("Foo2", 1, 8, TraceMarker::StartSync),
            new_trace("Foo2", 1, 9, TraceMarker::StartSync),
            new_trace("Foo2", 1, 10, TraceMarker::StartSync),
            new_trace("", 1, 11, TraceMarker::EndSync),
            new_trace("", 1, 12, TraceMarker::EndSync),
            new_trace("", 1, 13, TraceMarker::EndSync),
            new_trace("Foo2", 1, 14, TraceMarker::StartSync),
            new_trace("", 1, 15, TraceMarker::EndSync),
            new_trace("", 1, 16, TraceMarker::EndSync), // Foo ends here
            new_trace("Foo2", 1, 17, TraceMarker::StartSync),
            new_trace("Foo2", 1, 18, TraceMarker::StartSync),
            new_trace("Foo2", 1, 19, TraceMarker::StartSync),
            new_trace("", 1, 20, TraceMarker::EndSync),
            new_trace("", 1, 21, TraceMarker::EndSync),
            new_trace("", 1, 22, TraceMarker::EndSync),
            new_trace("", 1, 23, TraceMarker::EndSync),
        ];

        let res = find_all_spans(String::from("Foo"), &traces);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].start.timestamp.seconds, 1);
        assert_eq!(res[0].end.timestamp.seconds, 16);
    }
}
