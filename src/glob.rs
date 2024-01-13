// for test debugging
macro_rules! conditional_log {
    ($($arg:tt)*) => {
        #[cfg(test)]
        println!($($arg)*);
    };
}


/// Search given glob pattern under lined constraint.
///
/// # Arguments
///
/// * `text` - Target text to search. Any type with Index traint can be accepted.
/// * `pattern` - Glob pattern. E,g, ["abc","def"] means abc*def
/// * `eol` - End of line value. E.g., plain ASCII text case, \x0d \x0a are meaning.`
///
/// # Example
///
/// ```
/// let text = "abbabababcdef";
/// let result = lined_glob(&text,["abc","def"],['\x0d','\x0a']);
/// assert_eq!(result, (7,13));
/// ```
///
/// # Returns
///
/// This function returns the pair of usize means start and end.
///
pub fn lined_glob<T>(text:&[T],patterns:&Vec<&[T]>,eol:&[T]) -> Option<(usize,usize)>
where
    T: PartialEq + Eq
{
    let mut pattern_idx = 0;
    let mut pattern_elm_idx = 0;
    let mut line_start = 0;
    let mut line_end = 0;

    for (text_elm_idx,text_elm_val) in text.iter().enumerate() 

    {
        let selected_pattern = patterns[pattern_idx];
        match text_elm_val {
            // EOL make terminate all matching and go next text_pos
            x if eol.contains(x) => {
                line_start = text_elm_idx + 1;
                pattern_idx = 0;
                pattern_elm_idx = 0;
            },
            // the pattern is matched partially
            x if *x==selected_pattern[pattern_elm_idx] => {
                conditional_log!("char is match");
                // the pattern is matched complete
                if pattern_elm_idx == selected_pattern.len() - 1 {
                    conditional_log!("the pattern {} is end",pattern_idx);
                    // last pattern is last
                    if pattern_idx == patterns.len() - 1 {
                        conditional_log!("patterns are end");
                        // find next eol
                        line_end = 12345678;
                        for (i,v) in text[text_elm_idx+1..].iter().enumerate() {
                            if eol.contains(v) {
                                conditional_log!("found EOL");
                                line_end = i+text_elm_idx+2;
                            }
                        };
                        // no eol found, then end of text is end
                        if line_end == 12345678 {
                            line_end = text.len();
                        }
                        return Some((line_start,line_end));
                    } else {
                        // selected pattern matching is finished, go next pattern
                        conditional_log!("go to next pattern pattern index is {}", pattern_idx);
                        pattern_idx += 1;
                        pattern_elm_idx = 0;
                    }
                } else {
                    // continue selected pattern macthing
                    conditional_log!("non match chat");
                    pattern_elm_idx += 1;
                } 
            },
            // x doesn't match non glob mode
            _ => {
                pattern_elm_idx = 0;
            }
        }
    };
    None
}


#[cfg(test)]
mod tests {
    use super::*; // Import everything from the outer module

    #[test]
    fn test_oneline() {
        let text = [1,2,3];
        let pat_1 = [1,2];
        let pat = vec![&pat_1[..]];
        let eol = [0];
        assert_eq!(lined_glob(&text,&pat,&eol),Some((0,3)));
    }

    #[test]
    fn test_eol_simple() {
        let text = [1,2, 0, 3,4];
        let pat_1 = [1,2];
        let pat = vec![&pat_1[..]];
        let eol = [0];
        assert_eq!(lined_glob(&text,&pat,&eol[..]),Some((0,3)));
    }

    #[test]
    fn test_eol_addchars() {
        let text = [1,2, 3,4,0, 3];
        let pat_1 = [1,2];
        let pat = vec![&pat_1[..]];
        let eol = [0];
        assert_eq!(lined_glob(&text,&pat,&eol[..]),Some((0,5)));
    }

    #[test]
    fn test_eol_nextline() {
        let text = [1,1,0,1,2,0, 3];
        let pat_1 = [1,2];
        let pat = vec![&pat_1[..]];
        let eol = [0];
        assert_eq!(lined_glob(&text,&pat,&eol[..]),Some((3,6)));
    }

    #[test]
    fn test_two_pattern_without_gap() {
        let text = [1,1,0,1, 1, 1,2,0, 3];
        let pat_1 = [1,1];
        let pat_2 = [1,2];
        let pat = vec![&pat_1[..],&pat_2[..]];
        let eol = [0];
        assert_eq!(lined_glob(&text,&pat,&eol[..]),Some((3,8)));
    }

    #[test]
    fn test_two_pattern_with_gap() {
        let text = [1,1,0,1, 1, 5,6,7,1,2,0, 3];
        let pat_1 = [1,1];
        let pat_2 = [1,2];
        let pat = vec![&pat_1[..],&pat_2[..]];
        let eol = [0];
        assert_eq!(lined_glob(&text,&pat,&eol[..]),Some((3,11)));
    }
    /* 
    #[test]
    #[should_panic]
    fn test_add_overflow() {
        // This test is designed to fail
        add(i32::MAX, 1);
    }
    */
}