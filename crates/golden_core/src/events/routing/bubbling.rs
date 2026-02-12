pub enum BubblingPolicy {
    ParentOnly,
    UntilBoundary,
    MaxDepth(usize),
    ToRoot,
}
