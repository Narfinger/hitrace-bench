use crate::Trace;

pub(crate) fn find_span<'a>(string: String, traces: &'a [Trace]) -> Option<(&'a Trace, &'a Trace)> {
    let start_pos = traces.iter().position(|p| p.function==string).unwrap();
    let mut queue_size = 1;
    let start = traces.get(start_pos).unwrap();

    let mut pos = start_pos +1;
    println!("Start {:?}", pos);
    while pos!= traces.len() && queue_size!=0 {
        println!("Queue size {:?}", queue_size);
        let trace = traces.get(pos).unwrap();
        queue_size = match trace.trace_marker {
            crate::trace::TraceMarker::StartSync => { queue_size+1 },
            crate::trace::TraceMarker::EndSync => {
                println!("Finding ending marker at queue size {:?}", queue_size);
                queue_size -1 },
            crate::trace::TraceMarker::StartAsync => { queue_size +1 },
            crate::trace::TraceMarker::EndAsync => { queue_size -1 },
            crate::trace::TraceMarker::Dot => {queue_size},
        };
        pos +=1;
    }

    if pos == traces.len() {
        println!("Reached and");
        None
    } else {
        let end = traces.get(pos).unwrap();
        println!("Found {:?} {:?}", start, end);
        Some((start, end))

    }
}